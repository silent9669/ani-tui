class AniTui < Formula
  desc "A Netflix-inspired TUI for anime streaming"
  homepage "https://github.com/silent9669/ani-tui"
  url "https://github.com/silent9669/ani-tui/archive/refs/tags/v3.0.0.tar.gz"
  sha256 "7bb23bad293e0cbc019c33051140cc7c5d8575243644b8e3c8dad8deebf34bc0"
  license "MIT"
  head "https://github.com/silent9669/ani-tui.git", branch: "main"

  depends_on arch: :x86_64
  depends_on macos: [
    :sonoma, # macOS 14
    :sequoia, # macOS 15
  ]
  depends_on "rust" => :build
  depends_on "chafa"
  depends_on "mpv"

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    system "#{bin}/ani-tui", "--version"
  end
end
