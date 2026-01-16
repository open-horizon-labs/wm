class Wm < Formula
  desc "Working memory for AI coding assistants"
  homepage "https://github.com/cloud-atlas-ai/wm"
  url "https://github.com/open-horizon-labs/wm/archive/refs/tags/v0.3.2.tar.gz"
  sha256 "8ebd3a36953817459ae17f639aa38c5c3de58a6bd4b77502c048b639432bb6d4"
  license :cannot_represent

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match "wm", shell_output("#{bin}/wm --help")
  end
end
