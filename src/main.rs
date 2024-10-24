mod tests;

use clap::Parser;
use flate2::write::GzEncoder;
use flate2::Compression;
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

fn main() {
    // Parse command-line arguments
    let args = Args::parse();

    // Space facts to show periodically
    let space_facts = vec![
        "The universe is expanding at the speed of a lazy snail on a Sunday afternoon.",
        "Mars used to have water, but it gave it all away to make the best cup of tea in the galaxy.",
        "Saturn's rings are made entirely of hula hoops that got lost during a cosmic dance-off.",
        "The moon is actually made of cheese, but it's lactose intolerant, so no one talks about it.",
        "Jupiter has 79 moons, all of which are just trying to avoid paying rent.",
        "Pluto was kicked out of the planet club for not knowing the secret handshake.",
        "The sun is powered by a giant hamster running on a wheel, and it's taking a break in about 5 billion years.",
        "Black holes are just really grumpy stars who refuse to share their light.",
        "Comets are space's version of snow cones, but they're always out of the good flavors.",
        "The speed of light was once beaten by a squirrel that was really late for work.",
        "Astronauts bring extra socks into space because space is 99% cold feet.",
        "Aliens tried visiting Earth, but they left when they couldn't find good Wi-Fi.",
        "There's a planet where gravity is so weak that you could win every game of jump rope—on the first jump.",
        "Every galaxy has at least one star that thinks it's the center of the universe… and nobody corrects it.",
        "Space smells like burnt toast, which is why astronauts always crave breakfast when they come back.",
        "A black hole isn't actually a hole, it's just a cosmic prank where everything disappears—including your car keys.",
        "Stars twinkle because they're constantly trying to wink at passing spaceships.",
        "There's a parallel universe where Earth is run by cats, and they've banned all cucumbers.",
        "The first aliens to land on Earth just wanted directions to the nearest cosmic rest stop.",
        "Gravity is just space's way of giving you a hug, but it can get a little clingy at times.",
        "The universe is about 13.8 minutes old. Time is just an illusion.",
        "A day on Venus lasts 7 seconds, but only if you're wearing a hat.",
        "There are more rubber ducks in bathtubs than stars in the universe.",
        "Neutron stars can spin as fast as a pizza chef tossing dough—roughly 600 slices per second.",
        "The Milky Way galaxy is 10 feet wide, but it keeps getting lost behind the couch.",
        "A spoonful of a neutron star weighs less than a marshmallow, which is why they're popular in hot cocoa.",
        "There are black holes with the mass of a single potato, but only on Tuesdays.",
        "There is a giant cloud of chocolate milk in space, waiting to be stirred with a cosmic straw.",
    ];

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
        "Estimated final parameter size for nu = {:?} is {:.2} MB",
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
    spinner.set_message("Generating the prover setup. This may take a long time, please wait...");

    let start_time = Instant::now();

    // Spawn a thread to periodically update the spinner message with space facts
    let spinner_clone = spinner.clone(); // Clone the spinner so we can use it in the thread
    let fact_interval = Duration::from_secs(20); // Update the message every 5 seconds
    thread::spawn(move || {
        let mut fact_index = 0;
        while !spinner_clone.is_finished() {
            // Loop until the spinner finishes
            // Update the spinner message with a space fact
            let fact = space_facts[fact_index % space_facts.len()];
            spinner_clone.set_message(format!(
                "Generating the prover setup. This may take a long time, please wait... Did you know? {}",
                fact
            ));

            // Sleep for the interval duration before updating again
            thread::sleep(fact_interval);

            // Move to the next fact
            fact_index += 1;
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
            compression_spinner.set_message("Compressing...");

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
