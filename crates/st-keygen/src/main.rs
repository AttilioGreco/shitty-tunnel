use st_infra::crypto::keys::KeyPair;

fn main() {
    let kp = KeyPair::generate();
    println!("Private key: {}", kp.private_to_base64());
    println!("Public key:  {}", kp.public_to_base64());
}
