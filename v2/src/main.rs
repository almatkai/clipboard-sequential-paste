#[tokio::main(worker_threads = 2)]
async fn main() {
    if tezbar_extension_sdk::serve(clipboard_sequential_paste::Adapter::default())
        .await
        .is_err()
    {
        eprintln!("Clipboard Sequential Paste protocol stopped");
        std::process::exit(1);
    }
}
