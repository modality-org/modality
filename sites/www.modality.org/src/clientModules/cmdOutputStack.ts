const EXPAND_ICON = `
  <span class="cmd-expand__hit" aria-hidden="true">
    <svg class="cmd-expand__icon" viewBox="0 0 16 16" focusable="false">
      <path
        d="M6 3.25 11.25 8 6 12.75"
        fill="none"
        stroke="currentColor"
        stroke-width="1.8"
        stroke-linecap="round"
        stroke-linejoin="round"
      />
    </svg>
  </span>
`;

function ensureExpandIcon(btn: HTMLButtonElement): void {
  if (!btn.querySelector('svg.cmd-expand__icon')) {
    btn.innerHTML = EXPAND_ICON;
  }
}

let pairCount = 0;

function setExpanded(
  btn: HTMLButtonElement,
  cmd: HTMLElement,
  output: HTMLElement,
  expanded: boolean,
): void {
  btn.setAttribute('aria-expanded', String(expanded));
  btn.setAttribute(
    'aria-label',
    expanded ? 'Hide expected output' : 'Show expected output',
  );
  cmd.classList.toggle('cmd-expanded', expanded);
  output.classList.toggle('cmd-expanded', expanded);
}

function wireButton(
  btn: HTMLButtonElement,
  cmd: HTMLElement,
  output: HTMLElement,
): void {
  if (!output.id) {
    pairCount += 1;
    output.id = `cmd-output-${pairCount}`;
  }
  btn.setAttribute('aria-controls', output.id);
  btn.onclick = (event) => {
    event.preventDefault();
    setExpanded(btn, cmd, output, btn.getAttribute('aria-expanded') !== 'true');
  };
}

function enhanceCmdOutputPairs(): void {
  document.querySelectorAll('.cmd-expand').forEach((btn) => {
    const next = btn.nextElementSibling;
    if (!next?.classList.contains('language-bash')) {
      btn.remove();
    }
  });

  document.querySelectorAll<HTMLElement>('.theme-code-block.language-bash').forEach((cmd) => {
    const output = cmd.nextElementSibling as HTMLElement | null;
    if (!output?.classList.contains('language-output')) {
      return;
    }

    cmd.classList.add('cmd-has-output');
    output.classList.add('cmd-is-paired-output');

    const existing = cmd.previousElementSibling;
    if (existing instanceof HTMLButtonElement && existing.classList.contains('cmd-expand')) {
      ensureExpandIcon(existing);
      wireButton(existing, cmd, output);
      return;
    }

    const btn = document.createElement('button');
    btn.type = 'button';
    btn.className = 'cmd-expand';
    btn.setAttribute('aria-expanded', 'false');
    btn.setAttribute('aria-label', 'Show expected output');
    btn.innerHTML = EXPAND_ICON;
    wireButton(btn, cmd, output);
    cmd.parentNode?.insertBefore(btn, cmd);
  });
}

let observer: MutationObserver | null = null;

function watchMarkdown(): void {
  const run = () => {
    observer?.disconnect();
    enhanceCmdOutputPairs();
    const root = document.querySelector('.theme-doc-markdown');
    if (!root) {
      return;
    }
    observer = new MutationObserver(run);
    observer.observe(root, {childList: true, subtree: true});
  };
  run();
}

export function onRouteDidUpdate(): void {
  requestAnimationFrame(() => watchMarkdown());
}
