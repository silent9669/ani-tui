class AniTui < Formula
  desc "A Netflix-inspired TUI for anime streaming"
  homepage "https://github.com/silent9669/ani-tui"
  url "https://github.com/silent9669/ani-tui/archive/refs/tags/v3.5.0.tar.gz"
  sha256 "6ed8c955fa41071dcb25c0c685061a4c50c453e620da541d66dd9716b45592f6"
  license "MIT"
  head "https://github.com/silent9669/ani-tui.git", branch: "main"

  depends_on "rust" => :build
  depends_on "chafa"
  depends_on "mpv"

  def install
    system "cargo", "install", *std_cargo_args
  end

  def caveats
    <<~EOS
      ani-tui has been installed!
      
      Usage:
        ani-tui              # Start the app
        ani-tui -q "naruto"  # Search immediately
      
      Run 'ani-tui --help' for more options.
    EOS
  end

  test do
    system "#{bin}/ani-tui", "--version"
  end
end