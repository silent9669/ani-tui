class AniTui < Formula
  desc "Anime TUI with image previews and terminal streaming"
  homepage "https://github.com/silent9669/ani-tui"
  url "https://github.com/silent9669/ani-tui/archive/refs/tags/v1.0.0.tar.gz"
  sha256 "PLACEHOLDER"  # Update after creating release
  license "GPL-3.0-or-later"
  head "https://github.com/silent9669/ani-tui.git", branch: "main"

  depends_on "bash"
  depends_on "curl"
  depends_on "fzf"
  depends_on "jq"
  depends_on "chafa" => :recommended
  depends_on "mpv" => :recommended

  def install
    # Install ani-tui script
    bin.install "macos/ani-tui" => "ani-tui"
    
    # Install ani-cli dependency (bundled)
    libexec.install "ani-tui/ani-cli"
    
    # Patch script to use libexec ani-cli
    inreplace bin/"ani-tui", 
      'ANI_CLI="${SCRIPT_DIR}/ani-tui/ani-cli"',
      "ANI_CLI=\"#{libexec}/ani-cli\""
  end

  def caveats
    <<~EOS
      ani-tui has been installed!
      
      For image previews, install chafa:
        brew install chafa
      
      For video playback, install mpv or iina:
        brew install mpv
        # or
        brew install --cask iina
      
      Usage:
        ani-tui              # Start TUI
        ani-tui --help       # Show help
    EOS
  end

  test do
    assert_match "ani-tui", shell_output("#{bin}/ani-tui --version")
  end
end
