import init, { hydrate } from '/pkg/etyloom.js';

try {
  await init({ module_or_path: '/pkg/etyloom_bg.wasm' });
  hydrate();
} catch (error) {
  console.error('Etyloom could not start its interface.', error);
  const main = document.getElementById('main');
  if (main) {
    const message = document.createElement('p');
    message.className = 'notice error';
    message.setAttribute('role', 'alert');
    message.textContent = 'The interface could not load. Reload the page or check the server assets. Your saved work is unchanged.';
    main.replaceChildren(message);
  }
}
