# ROE-09: Notes and Account-Data Synchronization

Prior: **already correctly owned; census and close**.

Core already owns the notes/ToDo account-data schema, normalization, CRUD, and
Matrix synchronization. React and SwiftUI own editors, interaction, drag and
reorder affordances, and native presentation.

## Bounded research question

Do both shipped clients consume the same Core schema and operations for
create/edit/delete/reorder/complete, or does either retain a second account-data
engine? Verify stable IDs, versioning/migration, limits, concurrent/offline
updates, tombstones, clock independence, message anchors, malformed remote
data, privacy, export, and downgrade behavior against current source/tests.

The expected deliverable is a close memo plus any missing shared schema or
two-client Synapse fixture. Do not create a second engine to address a native
reorder/editor UX defect.
