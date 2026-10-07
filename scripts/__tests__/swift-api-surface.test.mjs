import assert from "node:assert/strict";
import test from "node:test";

import { swiftApiSurface } from "../swift-api-surface.mjs";

const SAMPLE = `
// comment
public struct SyncStatusDto: Equatable, Hashable {
    public var readiness: String
    public var commandGate: String

    // Default memberwise initializers are never public by default, so we
    // declare one manually.
    public init(readiness: String, commandGate: String) {
        self.readiness = readiness
        self.commandGate = commandGate
    }
}
public struct FfiConverterTypeSyncStatusDto: FfiConverterRustBuffer {
    public static func read(from buf: inout (data: Data, offset: Data.Index)) throws -> SyncStatusDto {
        return SyncStatusDto(readiness: "{", commandGate: "}")
    }
}
public
enum SessionStatusError: Swift.Error, Equatable {
    case Failed(code: String, description: String
    )
}
open class SharedCore: SharedCoreProtocol, @unchecked Sendable {
    public func uniffiCloneHandle() -> UInt64 { return 0 }
open func zeta()async throws  -> String  {
    switch x { case .a: break }
}
open func alpha(roomId: String)async throws   {
}
}
public protocol IosSecretVault: AnyObject, Sendable {
    func put(key: String, value: Data) throws
    func get(key: String) throws -> Data?
}
fileprivate struct Hidden { public var x: Int }
`;

test("keeps the public surface and drops FFI internals", () => {
  const surface = swiftApiSurface(SAMPLE);
  assert.match(surface, /public struct SyncStatusDto: Equatable, Hashable\n  public var readiness: String\n  public var commandGate: String\n  public init\(readiness: String, commandGate: String\)/);
  assert.match(surface, /public enum SessionStatusError: Swift.Error, Equatable\n  case Failed\(code: String, description: String\)/);
  assert.doesNotMatch(surface, /FfiConverter|uniffiCloneHandle|Hidden|case \.a/);
});

test("sorts class and protocol members but keeps record field order", () => {
  const surface = swiftApiSurface(SAMPLE);
  assert.match(surface, /open class SharedCore[^\n]*\n  open func alpha\(roomId: String\) async throws\n  open func zeta\(\) async throws -> String/);
  assert.match(surface, /public protocol IosSecretVault[^\n]*\n  func get\(key: String\) throws -> Data\?\n  func put/);
  const reordered = SAMPLE.replace(
    "public var readiness: String\n    public var commandGate: String",
    "public var commandGate: String\n    public var readiness: String"
  );
  assert.notEqual(swiftApiSurface(reordered), swiftApiSurface(SAMPLE));
});

test("is independent of top-level declaration order", () => {
  const blocks = SAMPLE.split("\nopen class");
  const swapped = `open class${blocks[1]}\n${blocks[0]}`;
  assert.equal(swiftApiSurface(swapped), swiftApiSurface(SAMPLE));
});
