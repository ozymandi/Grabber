// Help overlay (O'Tool pattern from O'Zometrix): opens from the help icon, closes with the
// close buttons, Esc or a click on the dimmed backdrop.

const overlay = document.getElementById('helpOverlay');
const openBtn = document.getElementById('helpBtn');

export const isHelpOpen = () => !overlay.hidden;

export function openHelp() {
    overlay.hidden = false;
    openBtn.setAttribute('aria-expanded', 'true');
    document.getElementById('helpClose').focus();
}

export function closeHelp() {
    if (overlay.hidden) return;
    overlay.hidden = true;
    openBtn.setAttribute('aria-expanded', 'false');
    openBtn.focus();
}

export function initHelp() {
    openBtn.addEventListener('click', openHelp);
    document.getElementById('helpClose').addEventListener('click', closeHelp);
    document.getElementById('helpCloseBottom').addEventListener('click', closeHelp);
    overlay.addEventListener('click', (e) => {
        if (e.target === overlay) closeHelp();
    });
    // Capture phase so Esc closes help before the app treats it as "exit".
    window.addEventListener(
        'keydown',
        (e) => {
            if (e.key === 'Escape' && isHelpOpen()) {
                e.preventDefault();
                e.stopImmediatePropagation();
                closeHelp();
            }
        },
        true,
    );
}
