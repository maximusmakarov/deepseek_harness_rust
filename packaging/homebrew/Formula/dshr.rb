class Dshr < Formula
  desc "Rust-native agent harness (TUI, headless, HTTP/SSE, ACP)"
  homepage "https://github.com/maximusmakarov/deepseek_harness_rust"
  head "https://github.com/maximusmakarov/deepseek_harness_rust.git", branch: "main"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args(path: "crates/cli")
  end

  test do
    assert_match "dshr", shell_output("#{bin}/dshr --version")
  end
end
