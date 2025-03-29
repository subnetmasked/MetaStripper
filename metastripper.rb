class Metastripper < Formula
  desc "A cross-platform command-line tool for removing privacy-sensitive metadata from files"
  homepage "https://github.com/subnetmasked/MetaStripper"
  url "https://github.com/subnetmasked/MetaStripper/archive/v0.1.3.tar.gz"
  sha256 "143cbf3b9415a229bb62c445705c45a87c2af15addb5d456fc9036c7ca5dfb50"

  depends_on "rust" => :build
  depends_on "ffmpeg" => :recommended

  def install
    system "cargo", "install", "--root", prefix, "--path", "."
  end

  test do
    system "#{bin}/metastripper", "--help"
  end
end 