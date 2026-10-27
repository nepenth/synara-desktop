import Foundation

/// The rules desktop's `remoteContent.ts` applies before the app loads
/// anything from a third party: HTTPS only, no credentials in the URL, and
/// no local, private or link-local host.
enum RemoteContentURL {
    private static let localhostNames: Set<String> = ["localhost", "localhost.", "0.0.0.0"]
    private static let localHostSuffixes = [
        ".localhost",
        ".local",
        ".localdomain",
        ".internal",
        ".lan",
        ".home.arpa",
    ]
    private static let privateIPv4Prefixes: [NSRegularExpression] = [
        #"^0\."#,
        #"^10\."#,
        #"^127\."#,
        #"^169\.254\."#,
        #"^172\.(1[6-9]|2\d|3[0-1])\."#,
        #"^192\.168\."#,
        #"^100\.(6[4-9]|[7-9]\d|1[01]\d|12[0-7])\."#,
    ].compactMap { try? NSRegularExpression(pattern: $0) }
    /// Hosts a WHATWG parser would read as an IPv4 number (`2130706433`,
    /// `0x7f.1`). Only the plain dotted-quad form is accepted.
    private static let numericHost = try? NSRegularExpression(
        pattern: #"^(0x[0-9a-f]*|[0-9]+)(\.(0x[0-9a-f]*|[0-9]+)){0,3}\.?$"#
    )
    private static let dottedQuad = try? NSRegularExpression(
        pattern: #"^(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}$"#
    )

    static func normalizeHost(_ hostname: String) -> String {
        var host = hostname.lowercased()
        if host.hasPrefix("[") { host.removeFirst() }
        if host.hasSuffix("]") { host.removeLast() }
        if host.hasSuffix(".") { host.removeLast() }
        return host
    }

    static func isPrivateIPv4(_ host: String) -> Bool {
        let host = normalizeHost(host)
        return privateIPv4Prefixes.contains { matches($0, host) }
    }

    static func isPrivateIPv6(_ host: String) -> Bool {
        let host = normalizeHost(host)
        return host == "::1"
            || host == "::"
            || host.hasPrefix("fc")
            || host.hasPrefix("fd")
            || host.hasPrefix("fe80")
            || host.hasPrefix("::ffff:")
    }

    static func isLocalHostname(_ hostname: String) -> Bool {
        let host = normalizeHost(hostname)
        return localhostNames.contains(host) || localHostSuffixes.contains { host.hasSuffix($0) }
    }

    /// The parsed components of `value` when it is safe to load, else `nil`.
    static func safeComponents(_ value: String) -> URLComponents? {
        // A WHATWG parser would encode these; refusing them keeps one
        // string from meaning two URLs.
        guard value.isEmpty == false,
              value.unicodeScalars.contains(where: { CharacterSet.whitespacesAndNewlines.contains($0)
                  || CharacterSet.controlCharacters.contains($0) }) == false,
              let components = URLComponents(string: value),
              components.scheme?.lowercased() == "https",
              (components.user ?? "").isEmpty,
              (components.password ?? "").isEmpty,
              let rawHost = components.host,
              rawHost.isEmpty == false
        else {
            return nil
        }
        let host = normalizeHost(rawHost)
        guard host.isEmpty == false,
              isLocalHostname(host) == false,
              isPrivateIPv4(host) == false
        else {
            return nil
        }
        if host.contains(":"), isPrivateIPv6(host) {
            return nil
        }
        if let numericHost, matches(numericHost, host) {
            guard let dottedQuad, matches(dottedQuad, host) else {
                return nil
            }
        }
        return components
    }

    static func isSafeHTTPS(_ value: String) -> Bool {
        safeComponents(value) != nil
    }

    /// `value` unchanged when it is safe to load.
    static func safe(_ value: String) -> String? {
        isSafeHTTPS(value) ? value : nil
    }

    /// The lowercased host of a safe URL.
    static func safeHost(_ value: String) -> String? {
        safeComponents(value)?.host.map(normalizeHost)
    }

    private static func matches(_ expression: NSRegularExpression, _ value: String) -> Bool {
        expression.firstMatch(in: value, range: NSRange(value.startIndex..., in: value)) != nil
    }
}

/// JSON reads with JavaScript's type checks: a boolean is not a number, an
/// empty string is absent.
enum JSONValue {
    static func object(_ value: Any?) -> [String: Any]? {
        value as? [String: Any]
    }

    static func string(_ value: Any?) -> String? {
        guard let string = value as? String, string.isEmpty == false else {
            return nil
        }
        return string
    }

    static func number(_ value: Any?) -> Double? {
        guard let number = value as? NSNumber, isBoolean(number) == false else {
            return nil
        }
        let double = number.doubleValue
        return double.isFinite ? double : nil
    }

    static func isTrue(_ value: Any?) -> Bool {
        guard let number = value as? NSNumber, isBoolean(number) else {
            return false
        }
        return number.boolValue
    }

    /// `String(n)` for the integral ids Klipy sends.
    static func numberText(_ value: Double) -> String {
        if value == value.rounded(), abs(value) < 1e15 {
            return String(Int64(value))
        }
        return String(value)
    }

    private static func isBoolean(_ number: NSNumber) -> Bool {
        CFGetTypeID(number) == CFBooleanGetTypeID()
    }
}

/// Requests to third parties (Klipy, GIPHY, X): no cookies, no shared cache,
/// no credentials, and no redirects, so a response can only come from the
/// host that was asked.
enum RemoteFetch {
    static let session: URLSession = {
        let configuration = URLSessionConfiguration.ephemeral
        configuration.httpCookieAcceptPolicy = .never
        configuration.httpShouldSetCookies = false
        configuration.httpCookieStorage = nil
        configuration.urlCredentialStorage = nil
        configuration.urlCache = nil
        configuration.requestCachePolicy = .reloadIgnoringLocalCacheData
        configuration.timeoutIntervalForRequest = 15
        configuration.timeoutIntervalForResource = 30
        configuration.httpMaximumConnectionsPerHost = 4
        return URLSession(configuration: configuration, delegate: NoRedirectDelegate(), delegateQueue: nil)
    }()

    private final class NoRedirectDelegate: NSObject, URLSessionTaskDelegate {
        func urlSession(
            _ session: URLSession,
            task: URLSessionTask,
            willPerformHTTPRedirection response: HTTPURLResponse,
            newRequest request: URLRequest,
            completionHandler: @escaping (URLRequest?) -> Void
        ) {
            completionHandler(nil)
        }
    }

    enum FetchError: Error {
        case unavailable
        case tooLarge
    }

    /// The body of a safe HTTPS `url`, refused past `maxBytes` and on any
    /// non-2xx answer. `acceptedTypes` are content-type prefixes.
    static func data(
        from url: URL,
        maxBytes: Int,
        accept: String? = nil,
        acceptedTypes: [String]? = nil,
        timeout: TimeInterval? = nil,
        session: URLSession = session
    ) async throws -> Data {
        guard RemoteContentURL.isSafeHTTPS(url.absoluteString) else {
            throw FetchError.unavailable
        }
        var request = URLRequest(url: url)
        request.httpMethod = "GET"
        if let accept {
            request.setValue(accept, forHTTPHeaderField: "Accept")
        }
        if let timeout {
            request.timeoutInterval = timeout
        }
        let (bytes, response) = try await session.bytes(for: request)
        guard let http = response as? HTTPURLResponse, (200 ..< 300).contains(http.statusCode) else {
            throw FetchError.unavailable
        }
        if let acceptedTypes {
            let type = (http.value(forHTTPHeaderField: "Content-Type") ?? "").lowercased()
            guard acceptedTypes.contains(where: { type.hasPrefix($0) }) else {
                throw FetchError.unavailable
            }
        }
        if http.expectedContentLength > Int64(maxBytes) {
            throw FetchError.tooLarge
        }
        var data = Data()
        data.reserveCapacity(Int(max(0, min(http.expectedContentLength, Int64(maxBytes)))))
        for try await byte in bytes {
            data.append(byte)
            if data.count > maxBytes {
                throw FetchError.tooLarge
            }
        }
        return data
    }
}
