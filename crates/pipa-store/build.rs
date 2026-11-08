// `sqlx::migrate!` menanam berkas migrasi saat kompilasi, tetapi Cargo tidak tahu folder itu berubah.
// Tanpa ini, menambah berkas migrasi baru tidak memicu kompilasi ulang dan migrasi itu tidak ikut terpasang.
fn main() {
    println!("cargo:rerun-if-changed=migrations");
}
