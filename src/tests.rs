#[cfg(test)]
mod tests {
    use flate2::read::GzDecoder; // Import GzDecoder to handle .gz files
    use proof_of_sql::proof_primitive::dory::{ProverSetup, PublicParameters};
    use std::{fs::File, io::BufReader, path::Path};
    use tar::Archive;

    // Helper function to untar and decompress a .tar.gz file into the current directory
    fn untar_gz_file(tar_gz_path: &str) -> std::io::Result<()> {
        let tar_gz_file = File::open(tar_gz_path)?; // Open the .tar.gz file
        let tar = GzDecoder::new(BufReader::new(tar_gz_file)); // Decompress the .tar.gz file
        let mut archive = Archive::new(tar); // Create a tar archive from the decompressed file
        archive.unpack(".")?; // Extract the files into the current directory
        Ok(())
    }

    #[test]
    fn test_untar_and_recreate_prover_setup() {
        // Step 1: Untar the .tar.gz archive
        let tar_gz_file_path = "dory-params.tar.gz";
        untar_gz_file(tar_gz_file_path).expect("Failed to untar the .tar.gz file");

        // Step 2: Read the public_parameters.bin
        let public_params_path = Path::new("public_parameters.bin");
        let public_parameters = PublicParameters::load_from_file(public_params_path)
            .expect("Failed to read public parameters");

        // Step 3: Read the blitzar_handle.bin
        let blitzar_handle_path = "blitzar_handle.bin";
        let blitzar_handle = blitzar::compute::MsmHandle::new_from_file(blitzar_handle_path);

        // Step 4: Recreate the ProverSetup using from_handle_and_params
        let _prover_setup = ProverSetup::from_public_parameters_and_blitzar_handle(
            &public_parameters,
            blitzar_handle,
        );

        // Clean up extracted files
        std::fs::remove_file(public_params_path).expect("Failed to delete public_parameters.bin");
        std::fs::remove_file(blitzar_handle_path).expect("Failed to delete blitzar_handle.bin");
    }
}
