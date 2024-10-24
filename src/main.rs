mod tests;

use clap::Parser;
use flate2::write::GzEncoder;
use flate2::Compression;
use indicatif::{ProgressBar, ProgressStyle};
use proof_of_sql::proof_primitive::dory::{ProverSetup, PublicParameters};
use rand::{Rng, SeedableRng};
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

    let space_facts = facts();

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
            // Generate a random index within the bounds of the space_facts vector
            let fact_index = rand::thread_rng().gen_range(0..space_facts.len());
            let fact = &space_facts[fact_index];

            // Update the spinner message with a randomly selected space fact
            spinner_clone.set_message(format!(
                "Generating public parameters for the SxT network. This may take a long time, please wait...\n  Did you know? {}",
                fact
            ));

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

fn facts() -> Vec<String> {
    // Space facts to show periodically
    let space_facts = vec![
        String::from("The Sun is 330,330 times larger than Earth.\n"),
        String::from("Volcano-ologists are experts in the study of volcanoes.\n"),
        String::from(
            "If you have trouble with simple counting, use the following mnemonic device: \n\
    one comes before two comes before 60 comes after 12 comes before \n\
    six trillion comes after 504. This will make your earlier counting difficulties\n\
    seem like no big deal.\n",
        ),
        String::from("The average adult body contains half a pound of salt.\n"),
        String::from(
            "The first person to prove that cow's milk is drinkable was very, very thirsty.\n",
        ),
        String::from("The atomic weight of Germanium is seven two point six four.\n"),
        String::from(
            "An ostrich's eye is bigger than its brain. Its brain size is 59.26 mm,\n\
    while its eye is 50.8 mm.\n",
        ),
        String::from("Humans can survive underwater. But not for very long.\n"),
        String::from("Polymerase I polypeptide A is a human gene. Shortened as POLR1C.\n"),
        String::from("Iguanas can stay underwater for twenty-eight point seven minutes.\n"),
        String::from("The moon orbits the Earth every 27.32 days.\n"),
        String::from("The billionth digit of Pi is 9.\n"),
        String::from("A gallon of water weighs 8.34 pounds.\n"),
        String::from("Hot water freezes quicker than cold water.\n"),
        String::from("Honey does not spoil. Instead, it will crystalize.\n"),
        String::from("A nanosecond lasts one billionth of a second.\n"),
        String::from(
            "According to Norse legend, thunder god Thor's chariot was pulled across the\n\
    sky by two goats.\n",
        ),
        String::from(
            "Tungsten has the highest melting point of any metal, at 3,410 degrees Celsius.\n",
        ),
        String::from(
            "The value of Pi is the ratio of any circle's circumference to its diameter in\n\
    Euclidean space.\n",
        ),
        String::from(
            "In 1879, Sandford Fleming first proposed the adoption of worldwide\n\
    standardized time zones at the Royal Canadian Institute.\n",
        ),
        String::from(
            "89% of magic tricks are not magic. Technically, they are sorcery.\n\
    The other 11% of magic tricks are probably also not magic.\n",
        ),
        String::from(
            "The plural of surgeon general is surgeons general. The past tense of\n\
    surgeons general is surgeonsed general.\n",
        ),
        String::from(
            "Edmund Hillary, the first person to climb Mount Everest,\n\
    did so accidentally while chasing a bird.\n",
        ),
        String::from(
            "Diamonds are made when coal is put under intense pressure. Diamonds put under\n\
    intense pressure become foam pellets, commonly used today as packing material.\n",
        ),
        String::from(
            "Halley's Comet can be viewed orbiting Earth every seventy-six years.\n\
    For the other seventy-five, it retreats to the heart of the sun,\n\
    where it hibernates undisturbed.\n",
        ),
        String::from(
            "In Greek myth, Prometheus stole fire from the Gods and gave it to humankind.\n\
    The jewelry he kept for himself.\n",
        ),
        String::from(
            "Pants were invented by sailors in the sixteenth century to avoid Poseidon's wrath.\n",
        ),
        String::from(
            "William Shakespeare did not exist. His plays were masterminded in 1589 by\n\
    Francis Bacon, who used an Ouija board to conjure play-writing ghosts.\n",
        ),
        String::from(
            "The automobile brake was not invented until 1895. Before this, someone had to\n\
    remain in the car at all times, driving in circles until passengers\n\
    returned from their errands.\n",
        ),
        String::from(
            "Before the Wright Brothers invented the airplane, anyone wanting to fly\n\
    anywhere was required to eat 200 pounds of helium.\n",
        ),
        String::from(
            "Before the invention of scrambled eggs in 1912, the typical breakfast was either\n\
    whole eggs still in the shell or scrambled rocks.\n",
        ),
        String::from("To make a photocopier, simply photocopy a mirror.\n"),
        String::from("Fact: Gigabrain is very handsome.\n"),
        String::from("Fact not found.\n"),
        String::from("Error. Error. Error. File not found.\n"),
        String::from("Error. Error. Error. Fact not found.\n"),
        String::from(
            "Warning, parameter corruption detec- Rats are regarded as the most handsome rodent.\n",
        ),
    ];
    space_facts
}
