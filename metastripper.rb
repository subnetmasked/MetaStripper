class Metastripper < Formula
  desc "A cross-platform command-line tool for removing privacy-sensitive metadata from files"
  homepage "https://github.com/subnetmasked/MetaStripper"
  url "https://github.com/subnetmasked/MetaStripper/archive/v0.1.0.tar.gz"
  sha256 "2936bb599d1af7cdbcd94121d362ab0adc7d8dad212fd5d5bdd1e2744d96e4b6"

  depends_on "rust" => :build
  depends_on "ffmpeg" => :recommended

  def install
    system "cargo", "install", "--root", prefix, "--path", "."
  end

  test do
    system "#{bin}/metastripper", "--help"
  end
end 