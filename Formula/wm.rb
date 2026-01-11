class Wm < Formula
  desc "Working memory for AI coding assistants"
  homepage "https://github.com/cloud-atlas-ai/wm"
  url "https://github.com/open-horizon-labs/wm/archive/refs/tags/v0.2.3.tar.gz"
  sha256 "324594a35eb7f223fbd635a7f171441a16f1bc14b77f0d23c3c3e98cf1f11ae5"
  license :cannot_represent

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match "wm", shell_output("#{bin}/wm --help")
  end
end
