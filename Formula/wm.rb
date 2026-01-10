class Wm < Formula
  desc "Working memory for AI coding assistants"
  homepage "https://github.com/cloud-atlas-ai/wm"
  url "https://github.com/cloud-atlas-ai/wm/archive/refs/tags/v0.2.2.tar.gz"
  sha256 "5ad56acc12d24a2c7d62d69df946f3b3a0a857aff37014a6b11e3e84c715e475"
  license :cannot_represent

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match "wm", shell_output("#{bin}/wm --help")
  end
end
