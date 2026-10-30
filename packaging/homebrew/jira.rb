class Jira < Formula
  desc "Modern developer-friendly Jira Cloud CLI built in Rust using Jira REST API v3"
  homepage "https://github.com/GBouerat/jira-cli"
  version "0.1.0"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/GBouerat/jira-cli/releases/download/v#{version}/jira-aarch64-apple-darwin.tar.gz"
      # sha256 "<checksum-from-SHA256SUMS.txt>"
    else
      url "https://github.com/GBouerat/jira-cli/releases/download/v#{version}/jira-x86_64-apple-darwin.tar.gz"
      # sha256 "<checksum-from-SHA256SUMS.txt>"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/GBouerat/jira-cli/releases/download/v#{version}/jira-aarch64-unknown-linux-gnu.tar.gz"
      # sha256 "<checksum-from-SHA256SUMS.txt>"
    else
      url "https://github.com/GBouerat/jira-cli/releases/download/v#{version}/jira-x86_64-unknown-linux-gnu.tar.gz"
      # sha256 "<checksum-from-SHA256SUMS.txt>"
    end
  end

  def install
    bin.install "jira"
    generate_completions_from_executable(bin/"jira", "completions", shells: [:bash, :zsh])
  end

  test do
    assert_match "jira #{version}", shell_output("#{bin}/jira --version")
  end
end
