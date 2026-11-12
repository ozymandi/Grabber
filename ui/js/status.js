// Bottom status row: indicator dot + text, full text mirrored into `title` (O'Tool rule).

const dot = document.getElementById('statusDot');
const text = document.getElementById('status');

const KINDS = ['info', 'success', 'error'];

/**
 * Shows a status message. `kind` is 'info', 'success' or 'error'. O'Tool has no
 * separate warning colour, so warnings use 'error'.
 */
export function setStatus(message, kind = 'info') {
    text.textContent = message;
    text.title = message;
    for (const k of KINDS) dot.classList.toggle(`status-indicator--${k}`, k === kind);
}
