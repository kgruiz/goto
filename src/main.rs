fn main() {

    if let Err(error) = goto::Run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
