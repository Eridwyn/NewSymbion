// Quick script to create initial user with bcrypt hash
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    username: String,
    password_hash: String,
    role: String,
}

fn main() {
    // Using bcrypt cost 10 (same as kernel)
    let password_hash = bcrypt::hash("admin123", 10).expect("Failed to hash password");

    let admin_user = User {
        username: "admin".to_string(),
        password_hash,
        role: "admin".to_string(),
    };

    let users = vec![admin_user];
    let json = serde_json::to_string_pretty(&users).expect("Failed to serialize");

    fs::write("users.json", json).expect("Failed to write users.json");

    println!("✅ users.json created with admin user");
    println!("   Username: admin");
    println!("   Password: admin123");
    println!("   Role: admin");
}
