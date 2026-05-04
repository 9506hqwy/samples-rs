use aes::cipher::{
    BlockModeDecrypt, BlockModeEncrypt, KeyIvInit, block_padding::Error, block_padding::Pkcs7,
};
use clap::{Parser, Subcommand};
use rand::Rng;
use rand::rngs::StdRng;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;
type Aes256CbcEnc = cbc::Encryptor<aes::Aes256>;

fn decrypt_payload(key: &[u8; 32], iv: &[u8; 16], payload: &[u8]) -> Result<Vec<u8>, Error> {
    let decryptor = Aes256CbcDec::new(key.into(), iv.into());
    decryptor.decrypt_padded_vec::<Pkcs7>(payload)
}

fn encrypt_payload(key: &[u8; 32], iv: &[u8; 16], payload: &[u8]) -> Vec<u8> {
    let encryptor = Aes256CbcEnc::new(key.into(), iv.into());
    encryptor.encrypt_padded_vec::<Pkcs7>(payload)
}

fn decrypt(secret: &Path, input: Option<&Path>, output: Option<&Path>) -> Result<(), io::Error> {
    let (key, iv) = read_secret_file(secret)?;

    let payload = read_input(input)?;

    let decrypted_payload = decrypt_payload(&key, &iv, &payload).unwrap();

    write_output(output, &decrypted_payload)?;

    Ok(())
}

fn encrypt(secret: &Path, input: Option<&Path>, output: Option<&Path>) -> Result<(), io::Error> {
    let (key, iv) = read_secret_file(secret)?;

    let payload = read_input(input)?;

    let encrypted_payload = encrypt_payload(&key, &iv, &payload);

    write_output(output, &encrypted_payload)?;

    Ok(())
}

fn generate_secret_file(path: &Path) -> Result<(), io::Error> {
    let mut rng: StdRng = rand::make_rng();

    let mut secret = [0u8; 48];
    rng.fill_bytes(&mut secret);

    fs::write(path, secret)?;

    Ok(())
}

fn read_secret_file(path: &Path) -> Result<([u8; 32], [u8; 16]), io::Error> {
    let mut secret = File::open(path)?;

    let mut key = [0u8; 32];
    let mut iv = [0u8; 16];
    secret.read_exact(&mut key)?;
    secret.read_exact(&mut iv)?;

    Ok((key, iv))
}

fn read_input(input: Option<&Path>) -> Result<Vec<u8>, io::Error> {
    if let Some(input) = input {
        fs::read(input)
    } else {
        let mut buffer = Vec::new();
        io::stdin().read_to_end(&mut buffer)?;
        Ok(buffer)
    }
}

fn write_output(output: Option<&Path>, data: &[u8]) -> Result<(), io::Error> {
    if let Some(output) = output {
        fs::write(output, data)
    } else {
        io::stdout().write_all(data)
    }
}

#[derive(Subcommand)]
enum Commands {
    Decrypt {
        #[arg(long)]
        secret: PathBuf,

        #[arg(long)]
        input: Option<PathBuf>,

        #[arg(long)]
        output: Option<PathBuf>,
    },

    Encrypt {
        #[arg(long)]
        secret: PathBuf,

        #[arg(long)]
        input: Option<PathBuf>,

        #[arg(long)]
        output: Option<PathBuf>,
    },

    Generate {
        #[arg(long)]
        secret: PathBuf,
    },
}

#[derive(Parser)]
#[command(name = "aes256")]
#[command(version)]
#[command(about = "A sample AES-256-CBC encryptor/decryptor", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Decrypt {
            secret,
            input,
            output,
        } => {
            decrypt(&secret, input.as_deref(), output.as_deref()).unwrap();
        }
        Commands::Encrypt {
            secret,
            input,
            output,
        } => {
            encrypt(&secret, input.as_deref(), output.as_deref()).unwrap();
        }
        Commands::Generate { secret } => generate_secret_file(&secret).unwrap(),
    }
}
