class Metastripper < Formula
  desc "A cross-platform command-line tool for removing privacy-sensitive metadata from files"
  homepage "https://github.com/subnetmasked/MetaStripper"
  url "https://github.com/subnetmasked/MetaStripper/archive/v0.1.0.tar.gz"
  sha256 "" # You'll need to fill this in after creating the release

  depends_on "rust" => :build
  depends_on "ffmpeg" => :recommended

  def install
    system "cargo", "install", "--root", prefix, "--path", "."
  end

  test do
    system "#{bin}/metastripper", "--help"
  end
end 