use super::*;

#[tauri::command]
pub async fn matrix_room_key_transfer_status(
    core: State<'_, Arc<synara_core::Core>>,
) -> Result<NativeRoomKeyTransferStatus, MatrixAuthCommandError> {
    crate::bridge::room_key_status::room_key_transfer_status(core.inner().as_ref()).await
}

#[tauri::command]
pub async fn matrix_room_key_export(
    state: State<'_, MatrixAuthState>,
    mut passphrase: String,
) -> Result<NativeRoomKeyTransferResult, MatrixAuthCommandError> {
    let result = matrix_room_key_export_inner(&state, &passphrase).await;
    passphrase.zeroize();
    result
}

pub(super) async fn matrix_room_key_export_inner(
    state: &State<'_, MatrixAuthState>,
    passphrase: &str,
) -> Result<NativeRoomKeyTransferResult, MatrixAuthCommandError> {
    live_room_keys::require_passphrase(passphrase)?;
    let (client, generation, flow) = {
        let session = state.session.lock().await;
        let active = require_room_key_session(session.as_ref())?;
        (
            active.client.clone(),
            active.sync.session_generation(),
            Arc::clone(&active.room_key_transfer),
        )
    };
    let result = live_room_keys::export(&client, generation, &flow, passphrase).await?;
    require_current_room_key_generation(state, generation).await?;
    Ok(result)
}

#[tauri::command]
pub async fn matrix_room_key_import_select(
    state: State<'_, MatrixAuthState>,
) -> Result<Option<NativeRoomKeyFileSelection>, MatrixAuthCommandError> {
    let generation = {
        let session = state.session.lock().await;
        require_room_key_session(session.as_ref())?
            .sync
            .session_generation()
    };
    let picked = live_room_keys::pick_import_file().await;
    let Some((path, file_label)) = picked else {
        return Ok(None);
    };

    let mut session = state.session.lock().await;
    let active = require_room_key_session_mut(session.as_mut())?;
    if active.sync.session_generation() != generation {
        return Err(stale_room_key_generation_error());
    }
    active.next_room_key_import_selection_id =
        active.next_room_key_import_selection_id.saturating_add(1);
    let selection_id = active.next_room_key_import_selection_id;
    active.selected_room_key_import = Some(SelectedRoomKeyImport {
        selection_id,
        path,
        file_label: file_label.clone(),
    });
    Ok(Some(NativeRoomKeyFileSelection {
        selection_id,
        file_label,
    }))
}

#[tauri::command]
pub async fn matrix_room_key_import(
    state: State<'_, MatrixAuthState>,
    selection_id: u64,
    mut passphrase: String,
) -> Result<NativeRoomKeyTransferResult, MatrixAuthCommandError> {
    let result = matrix_room_key_import_inner(&state, selection_id, &passphrase).await;
    passphrase.zeroize();
    result
}

pub(super) async fn matrix_room_key_import_inner(
    state: &State<'_, MatrixAuthState>,
    selection_id: u64,
    passphrase: &str,
) -> Result<NativeRoomKeyTransferResult, MatrixAuthCommandError> {
    let (client, generation, flow, selected) = {
        let mut session = state.session.lock().await;
        let active = require_room_key_session_mut(session.as_mut())?;
        let selected = reserve_room_key_import_selection(
            &mut active.selected_room_key_import,
            selection_id,
            passphrase,
        )?;
        (
            active.client.clone(),
            active.sync.session_generation(),
            Arc::clone(&active.room_key_transfer),
            selected,
        )
    };
    let result = live_room_keys::import(&client, generation, &flow, &selected, passphrase).await;
    let result = match result {
        Ok(result) => result,
        Err(error) => {
            restore_room_key_import_selection(state, generation, selected).await;
            return Err(error);
        }
    };
    require_current_room_key_generation(state, generation).await?;
    Ok(result)
}

pub(super) fn reserve_room_key_import_selection(
    slot: &mut Option<SelectedRoomKeyImport>,
    selection_id: u64,
    passphrase: &str,
) -> Result<SelectedRoomKeyImport, MatrixAuthCommandError> {
    live_room_keys::require_passphrase(passphrase)?;
    if slot
        .as_ref()
        .is_none_or(|selected| selected.selection_id != selection_id)
    {
        return Err(MatrixAuthCommandError::new(
            "InvalidRequest",
            "Choose an encrypted room-key file before importing.",
            "v-crypto.5-import-selection-invalid",
        ));
    }
    slot.take().ok_or_else(|| {
        MatrixAuthCommandError::new(
            "InvalidRequest",
            "Choose an encrypted room-key file before importing.",
            "v-crypto.5-import-selection-invalid",
        )
    })
}

pub(super) async fn restore_room_key_import_selection(
    state: &State<'_, MatrixAuthState>,
    generation: u64,
    selected: SelectedRoomKeyImport,
) {
    let mut session = state.session.lock().await;
    let Some(active) = session.as_mut() else {
        return;
    };
    restore_reserved_room_key_import(
        generation,
        Some(active.sync.session_generation()),
        &mut active.selected_room_key_import,
        selected,
    );
}

pub(super) fn restore_reserved_room_key_import(
    expected_generation: u64,
    current_generation: Option<u64>,
    slot: &mut Option<SelectedRoomKeyImport>,
    selected: SelectedRoomKeyImport,
) -> bool {
    if current_generation != Some(expected_generation) || slot.is_some() {
        return false;
    }
    *slot = Some(selected);
    true
}
