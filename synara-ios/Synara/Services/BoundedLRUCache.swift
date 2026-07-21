import Foundation

struct BoundedLRUCache<Key: Hashable, Value> {
    let capacity: Int
    private var values: [Key: Value] = [:]
    private var leastToMostRecentKeys: [Key] = []

    init(capacity: Int) {
        self.capacity = max(0, capacity)
    }

    var count: Int {
        values.count
    }

    var keysInEvictionOrder: [Key] {
        leastToMostRecentKeys
    }

    mutating func value(forKey key: Key) -> Value? {
        guard let value = values[key] else {
            return nil
        }
        touch(key)
        return value
    }

    @discardableResult
    mutating func insert(_ value: Value, forKey key: Key) -> (key: Key, value: Value)? {
        guard capacity > 0 else {
            return (key, value)
        }

        values[key] = value
        touch(key)
        guard values.count > capacity,
              let evictedKey = leastToMostRecentKeys.first,
              let evictedValue = values.removeValue(forKey: evictedKey)
        else {
            return nil
        }
        leastToMostRecentKeys.removeFirst()
        return (evictedKey, evictedValue)
    }

    @discardableResult
    mutating func removeValue(forKey key: Key) -> Value? {
        leastToMostRecentKeys.removeAll { $0 == key }
        return values.removeValue(forKey: key)
    }

    mutating func removeAll(keepingCapacity: Bool = false) {
        values.removeAll(keepingCapacity: keepingCapacity)
        leastToMostRecentKeys.removeAll(keepingCapacity: keepingCapacity)
    }

    private mutating func touch(_ key: Key) {
        leastToMostRecentKeys.removeAll { $0 == key }
        leastToMostRecentKeys.append(key)
    }
}
