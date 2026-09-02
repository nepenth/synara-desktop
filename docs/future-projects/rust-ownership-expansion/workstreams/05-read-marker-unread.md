# ROE-05: Read-Marker and Unread Calculations

Prior: **bounded visibility-contract remainder**.

Core owns unread/count truth and Matrix receipt/read-marker writes. Desktop and
iOS necessarily observe viewport visibility, focus, navigation, app lifecycle,
and explicit user intent.

## Bounded research question

Is the typed contract by which a platform reports genuine visibility to Core
underspecified or inconsistent across clients? Census public/private receipts,
fully-read markers, marked-unread state, notification/highlight counts,
threads, late decryption, local echoes, and races among sync, pagination, room
changes, backgrounding, retry, and multiple devices.

Propose contract clarification only if the earliest divergence is between a
platform observation and Core authority. Ordering/property tests, focus and
visibility fixtures, offline/reconnect cases, two-client Synapse receipts, and
desktop/iOS lifecycle tests are the required evidence.

## Keep closed

Do not move viewport geometry, scroll position, focus detection, or badge
presentation into Core. Do not let presenters independently decide receipt
eligibility once observations cross the contract.
