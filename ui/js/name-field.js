// Workspace name field (2.1.2), used by the Atlas slot in RIP and the Workspace panel in ATLAS.
// An unsaved workspace can be named here: Enter or leaving the field commits, Esc reverts.
// A saved workspace is named after its .ripper file, so the field is plain read-only text that
// takes no focus and no selection.

const state = new WeakMap(); // input -> { value, commit }

/**
 * Sets the field's value and mode. `commit(name)` is called with a changed, non-empty name and
 * returns a promise; on failure the field shows the last value again.
 */
export function setNameField(input, { value, editable, file, commit }) {
    let s = state.get(input);
    if (!s) {
        s = { value, commit };
        state.set(input, s);
        input.addEventListener('mousedown', (e) => {
            if (input.readOnly) e.preventDefault();
        });
        input.addEventListener('keydown', (e) => {
            if (input.readOnly) return;
            if (e.key === 'Enter') {
                e.preventDefault();
                input.blur();
            } else if (e.key === 'Escape') {
                e.preventDefault();
                e.stopPropagation();
                input.value = s.value;
                input.blur();
            }
        });
        input.addEventListener('change', async () => {
            const name = input.value.trim();
            if (input.readOnly || !name || name === s.value) {
                input.value = s.value;
                return;
            }
            const ok = await s.commit(name);
            if (!ok) input.value = s.value;
        });
    }
    s.value = value;
    s.commit = commit;
    input.readOnly = !editable;
    input.tabIndex = editable ? 0 : -1;
    input.classList.toggle('name-locked', !editable);
    input.title = editable
        ? 'Name of the atlas workspace; it becomes the file name on SAVE'
        : `Saved as ${file}. Rename it with SAVE AS in ATLAS`;
    if (document.activeElement !== input) input.value = value;
}
