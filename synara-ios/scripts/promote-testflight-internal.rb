#!/usr/bin/env ruby
# frozen_string_literal: true

require "base64"
require "json"
require "net/http"
require "openssl"
require "pathname"
require "time"
require "uri"

class AppStoreConnectError < StandardError
  attr_reader :status, :payload

  def initialize(method, path, status, payload)
    @status = status
    @payload = payload
    super("ASC API #{method.upcase} #{path} failed with HTTP #{status}")
  end
end

SCRIPT_DIR = Pathname.new(__dir__)
PROJECT_DIR = SCRIPT_DIR.parent
PROJECT_FILE = PROJECT_DIR.join("Synara.xcodeproj", "project.pbxproj")
BASE_URL = "https://api.appstoreconnect.apple.com"

def read_project_setting(name)
  match = PROJECT_FILE.read.match(/#{Regexp.escape(name)} = ([^;]+);/)
  match && match[1].strip
end

KEY_PATH = ENV.fetch("SYNARA_ASC_KEY_PATH")
KEY_ID = ENV.fetch("SYNARA_ASC_KEY_ID")
ISSUER_ID = ENV.fetch("SYNARA_ASC_ISSUER_ID")
BUNDLE_ID = ENV.fetch("SYNARA_IOS_BUNDLE_ID", "com.whylandcreative.synara")
VERSION = ENV.fetch("SYNARA_IOS_VERSION", read_project_setting("MARKETING_VERSION"))
BUILD_NUMBER = ENV.fetch("SYNARA_IOS_BUILD", read_project_setting("CURRENT_PROJECT_VERSION"))
GROUP_NAME = ENV["SYNARA_ASC_BETA_GROUP_NAME"]

if VERSION.nil? || BUILD_NUMBER.nil?
  warn "Unable to resolve MARKETING_VERSION or CURRENT_PROJECT_VERSION."
  exit 1
end

def base64url(data)
  Base64.urlsafe_encode64(data).delete("=")
end

def jwt
  header = { alg: "ES256", kid: KEY_ID, typ: "JWT" }
  payload = {
    iss: ISSUER_ID,
    exp: Time.now.to_i + 20 * 60,
    aud: "appstoreconnect-v1"
  }
  signing_input = "#{base64url(JSON.generate(header))}.#{base64url(JSON.generate(payload))}"
  key = OpenSSL::PKey.read(File.read(KEY_PATH))
  der_signature = key.sign(OpenSSL::Digest::SHA256.new, signing_input)
  sequence = OpenSSL::ASN1.decode(der_signature)
  raw_signature = sequence.value.map do |integer|
    integer.value.to_s(2).rjust(32, "\x00")[-32, 32]
  end.join
  "#{signing_input}.#{base64url(raw_signature)}"
end

TOKEN = jwt

def request(method, path, body: nil)
  uri = URI("#{BASE_URL}#{path}")
  klass = case method
          when :get then Net::HTTP::Get
          when :post then Net::HTTP::Post
          else raise "Unsupported method: #{method}"
          end
  req = klass.new(uri)
  req["Authorization"] = "Bearer #{TOKEN}"
  req["Content-Type"] = "application/json"
  req.body = JSON.generate(body) if body
  res = Net::HTTP.start(uri.hostname, uri.port, use_ssl: true) { |http| http.request(req) }
  parsed = res.body && !res.body.empty? ? JSON.parse(res.body) : {}
  raise AppStoreConnectError.new(method, path, res.code.to_i, parsed) unless res.code.to_i.between?(200, 299)

  parsed
end

def get(path)
  request(:get, path)
end

def post(path, body)
  request(:post, path, body: body)
end

def add_build_to_group(build_id, group_id)
  relationship = {
    data: [
      {
        type: "builds",
        id: build_id
      }
    ]
  }
  post("/v1/betaGroups/#{group_id}/relationships/builds", relationship)
end

def add_group_to_build(build_id, group_id)
  relationship = {
    data: [
      {
        type: "betaGroups",
        id: group_id
      }
    ]
  }
  post("/v1/builds/#{build_id}/relationships/betaGroups", relationship)
end

def relationship_link(data, name)
  data.fetch("relationships").fetch(name).fetch("links").fetch("related")
end

def asc_error_detail(error)
  error.payload.fetch("errors", []).map { |entry| entry["detail"] || entry["title"] }.compact.join(" ")
end

app_response = get("/v1/apps?filter[bundleId]=#{URI.encode_www_form_component(BUNDLE_ID)}&limit=1")
app = app_response.fetch("data").first
raise "No App Store Connect app found for #{BUNDLE_ID}" unless app

builds_query = URI.encode_www_form(
  "filter[app]" => app.fetch("id"),
  "limit" => "50",
  "sort" => "-uploadedDate"
)
builds = get("/v1/builds?#{builds_query}").fetch("data")
build = builds.find { |candidate| candidate.fetch("attributes")["version"] == BUILD_NUMBER }
raise "No build found for #{BUNDLE_ID} #{VERSION} (#{BUILD_NUMBER}). Upload it first or wait for processing." unless build

attrs = build.fetch("attributes")
processing_state = attrs["processingState"] || "unknown"
puts "Found Synara #{VERSION} (#{BUILD_NUMBER}) build_id=#{build.fetch("id")} processingState=#{processing_state} expired=#{attrs["expired"]}"

unless attrs["processingState"].nil? || attrs["processingState"] == "VALID"
  puts "Build is not VALID yet. Wait for App Store Connect processing, then rerun this script."
  exit 2
end

groups_path = "#{relationship_link(app, "betaGroups")}?limit=200"
groups = get(URI(groups_path).request_uri).fetch("data")
groups = groups.select { |group| group.fetch("attributes")["isInternalGroup"] }
groups = groups.select { |group| group.fetch("attributes")["name"] == GROUP_NAME } if GROUP_NAME && !GROUP_NAME.empty?
group = groups.first
raise "No internal TestFlight beta group found#{GROUP_NAME ? " named #{GROUP_NAME}" : ""}." unless group

group_name = group.fetch("attributes")["name"]
puts "Using internal beta group #{group_name} group_id=#{group.fetch("id")}"

begin
  add_build_to_group(build.fetch("id"), group.fetch("id"))
  puts "Build assigned to internal TestFlight group."
rescue AppStoreConnectError => e
  detail = asc_error_detail(e)
  if e.status == 409
    puts "Build is already assigned to the internal TestFlight group."
  elsif e.status == 422 && detail.include?("Cannot add internal group to a build")
    begin
      add_group_to_build(build.fetch("id"), group.fetch("id"))
      puts "Build assigned to internal TestFlight group."
    rescue AppStoreConnectError => fallback_error
      fallback_detail = asc_error_detail(fallback_error)
      if fallback_error.status == 409
        puts "Build is already assigned to the internal TestFlight group."
      elsif fallback_error.status == 422 && fallback_detail.include?("Cannot add internal group to a build")
        puts "App Store Connect does not allow explicit build assignment to this internal group."
        puts "The build is VALID and should be available to internal testers after Apple's TestFlight propagation/compliance checks, or through a named internal group with automatic distribution enabled."
      else
        warn JSON.pretty_generate(fallback_error.payload)
        raise
      end
    end
  else
    warn JSON.pretty_generate(e.payload)
    raise
  end
end
