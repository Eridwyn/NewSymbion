// Génère un hash bcrypt pour un mot de passe
fn main() {
    let password = "Mark Sourire951";
    let hash = bcrypt::hash(password, 10).expect("Failed to hash");
    println!("{}", hash);
}
