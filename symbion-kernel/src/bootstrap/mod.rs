/**
 * SYMBION KERNEL - Bootstrap Modules
 *
 * ROLE: Split main() initialization into focused subsystem modules.
 * Each module exports an init function that takes its dependencies
 * and returns a subsystem struct with all initialized components.
 *
 * ORDERING (sequential, each depends on previous):
 *   auth → database → intelligence → decision → tasks → server
 */

pub mod auth;
pub mod database;
pub mod intelligence;
pub mod decision;
pub mod tasks;
pub mod server;
