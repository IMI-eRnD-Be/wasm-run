fn main() -> Result<(), Box<dyn std::error::Error>> {
    Err(backend::run().into())
}
