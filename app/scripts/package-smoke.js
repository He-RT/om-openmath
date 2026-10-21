// Runs only from the external package checks, inside the actual installed WebView.
(() => {
  const wait = async (read, message) => {
    const deadline = Date.now() + 90_000;
    while (Date.now() < deadline) {
      const value = read();
      if (value) return value;
      await new Promise(resolve => setTimeout(resolve, 100));
    }
    throw new Error(message);
  };
  window.__openmathPackageSmoke = { state: 'running' };
  void (async () => {
    if (!window.__TAURI_INTERNALS__) throw new Error('Expected the installed Tauri application');
    const add = await wait(() => document.querySelector('.add-math'), 'Workbench did not initialize');
    add.click();
    const editor = await wait(() => document.querySelector('.cm-content'), 'Math editor did not appear');
    editor.focus();
    if (!document.execCommand('insertText', false, 'solve(x^2 - 5*x + 6 = 0, x)'))
      throw new Error('Could not enter the source into the real editor');
    await new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
    document.querySelector('.run-cell').click();
    await wait(() => document.querySelector('.notebook-cell[data-status="Done"] .solution-chip'), 'The packaged calculation did not finish');
    const roots = [...document.querySelectorAll('.solution-chip')].map(node => node.textContent);
    if (roots.length !== 2 || !roots.some(text => text.includes('2')) || !roots.some(text => text.includes('3')))
      throw new Error('The packaged calculation did not produce roots 2 and 3: ' + JSON.stringify(roots));
    const version = await (await fetch(new URL('version.json', location.href))).json();
    if (version.version !== '0.1.0-pre-alpha.5') throw new Error('Wrong packaged version');
    window.__openmathPackageSmoke = { state: 'passed', version, roots, nativeContainer: true };
  })().catch(error => {
    window.__openmathPackageSmoke = { state: 'failed', error: String(error), text: document.body.innerText.slice(0, 4000) };
  });
})();
