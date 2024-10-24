mod tests;

use clap::Parser;
use flate2::{write::GzEncoder, Compression};
use indicatif::{ProgressBar, ProgressStyle};
use proof_of_sql::proof_primitive::dory::{ProverSetup, PublicParameters};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use std::{
    fs::{self, File},
    path::Path,
    thread,
    time::{Duration, Instant},
};
use tar::Builder;

// Command-line argument parser structure
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The value for `nu` (number of public parameters)
    #[arg(short, long, default_value_t = 15)]
    nu: usize,
}

// Function to calculate the estimated file size based on nu
fn estimated_file_size(nu: usize) -> f64 {
    let base_size = 6.3; // 6.3 MB for nu = 4
    if nu >= 4 {
        base_size * 2_f64.powi((nu as i32) - 4)
    } else {
        base_size
    }
}

fn print_banner() {
    let banner = r#"
     _____     ______   ____                             ______                  
    / ___/_  _/_  __/  / __ \____ __________ _____ ___  / ____/__  ____          
    \__ \| |/_// /    / /_/ / __ `/ ___/ __ `/ __ `__ \/ / __/ _ \/ __ \         
   ___/ />  < / /    / ____/ /_/ / /  / /_/ / / / / / / /_/ /  __/ / / /         
  /____/_/|_|/_/    /_/    \__,_/_/   \__,_/_/ /_/ /_/\____/\___/_/ /_/          

  Space and Time® ParamGen v1.0
    "#;
    println!("{}", banner);
}

fn main() {
    print_banner();
    // Parse command-line arguments
    let args = Args::parse();

    // Convert the seed string to bytes and create a seeded RNG
    let seed_bytes = "SpaceAndTime"
        .bytes()
        .chain(std::iter::repeat(0u8))
        .take(32)
        .collect::<Vec<_>>()
        .try_into()
        .expect("collection is guaranteed to contain 32 elements");
    let mut rng = ChaCha20Rng::from_seed(seed_bytes);

    // Calculate and print the estimated file size
    let estimated_size_mb = estimated_file_size(args.nu);
    println!(
        "  Calculated public parameter size for nu = {:?} is {:.2} MB\n",
        args.nu, estimated_size_mb
    );

    // Use the `nu` value from the command-line argument
    let public_parameters = PublicParameters::rand(args.nu, &mut rng);

    // Initialize a spinner using ProgressBar
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    spinner.enable_steady_tick(Duration::from_millis(100));

    // Set initial message
    spinner.set_message(
        "Generating parameters for the SxT network.\nThis may take a long time, please wait...",
    );

    let start_time = Instant::now();

    // Spawn a thread to periodically update the spinner message with space facts
    let spinner_clone = spinner.clone(); // Clone the spinner so we can use it in the thread
    let fact_interval = Duration::from_secs(20); // Update the message every 5 seconds
    thread::spawn(move || {
        while !spinner_clone.is_finished() {
            // Update the spinner message with a randomly selected space fact
            spinner_clone.set_message("Generating public parameters for the SxT network. This may take a long time, please wait...\n".to_string());

            // Sleep for the interval duration before updating again
            thread::sleep(fact_interval);
        }
    });

    let prover_setup = ProverSetup::from(&public_parameters);

    // Stop the spinner once the operation is complete
    spinner.finish_with_message("Prover setup complete.");
    let duration = start_time.elapsed();
    println!("Generated prover setup in {:.2?}", duration);

    let result = public_parameters.save_to_file(Path::new("public_parameters.bin"));
    match result {
        Ok(_) => {
            // Write the blitzar handle to a .bin file
            let file_path = "blitzar_handle.bin";
            let blitzar_handle = prover_setup.blitzar_handle();
            blitzar_handle.write(file_path);

            // Create a new spinner for the compression phase
            let compression_spinner = ProgressBar::new_spinner();
            compression_spinner.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.green} {msg}")
                    .unwrap(),
            );
            compression_spinner.enable_steady_tick(Duration::from_millis(100));
            compression_spinner.set_message("Setup complete! Compressing...");

            // Start compression
            let tar_gz_file_path = "dory-params.tar.gz";
            let tar_gz_file = File::create(tar_gz_file_path).expect("Failed to create tar.gz file");
            let enc = GzEncoder::new(tar_gz_file, Compression::default());

            let mut tar_builder = Builder::new(enc);

            // Add both files to the tarball
            tar_builder
                .append_path("public_parameters.bin")
                .expect("Failed to add public_parameters.bin to the tar file");
            tar_builder
                .append_path("blitzar_handle.bin")
                .expect("Failed to add blitzar_handle.bin to the tar file");

            // Finalize the tar archive and compression
            tar_builder
                .finish()
                .expect("Failed to finalize the tar.gz file");

            // Stop the compression spinner
            compression_spinner.finish_with_message("Compression complete.");

            // Remove the .bin files after archiving
            fs::remove_file("public_parameters.bin")
                .expect("Failed to remove public_parameters.bin");
            fs::remove_file(file_path).expect("Failed to remove blitzar_handle.bin");

            println!("Temporary .bin files removed.");
        }
        Err(_) => println!("Failed to save parameters, aborting."),
    }
}
