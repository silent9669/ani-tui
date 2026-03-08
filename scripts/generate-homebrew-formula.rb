#!/usr/bin/env ruby

require 'erb'
require 'digest'
require 'open-uri'

if ARGV.empty?
  puts "Usage: #{$0} VERSION"
  puts "Example: #{$0} v3.6.1"
  exit 1
end

version = ARGV[0]
version_without_v = version.gsub(/^v/, '')

url = "https://github.com/silent9669/ani-tui/archive/refs/tags/#{version}.tar.gz"

puts "Downloading #{url}..."
sha256 = Digest::SHA256.hexdigest(URI.open(url).read)
puts "SHA256: #{sha256}"

template_path = File.join(__dir__, '..', 'packaging', 'homebrew', 'ani-tui.rb.template')
if !File.exist?(template_path)
  puts "Error: Template not found at #{template_path}"
  exit 1
end

template = ERB.new(File.read(template_path), trim_mode: '-')
formula = template.result(binding)

IO.popen(['ruby', '-c'], 'r+') do |io|
  io.write(formula)
  io.close_write
  result = io.read
  unless result.empty?
    puts "Ruby syntax error:"
    puts result
    exit 1
  end
end

output_path = 'ani-tui.rb'
if ARGV[1]
  output_path = ARGV[1]
end

File.write(output_path, formula)
puts ""
puts "✓ Generated formula: #{output_path}"
puts ""
puts "Validate with:"
puts "  brew audit --strict --online #{output_path}"
puts "  brew install --build-from-source #{output_path}"
puts "  brew test ani-tui"
