class Wm < Formula
  desc "Working memory for AI coding assistants"
  homepage "https://github.com/cloud-atlas-ai/wm"
  url "https://github.com/cloud-atlas-ai/wm/archive/refs/tags/v0.1.2.tar.gz"
  sha256 "f8d82f98f2af204cf6cf5a88a772e4aba9a4960aeda2e6fc37f168165d4d0100"
  license :cannot_represent

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match "wm", shell_output("#{bin}/wm --help")
  end
end
