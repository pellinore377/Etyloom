(() => {
  let preference = null;
  try { preference = localStorage.getItem('etyloom.theme'); }
  catch (error) { console.warn('Theme preference is unavailable; using the system theme.', error); }
  const system = matchMedia('(prefers-color-scheme: dark)');
  const apply = () => document.documentElement.setAttribute('data-theme', preference === 'light' || preference === 'dark' ? preference : system.matches ? 'dark' : 'light');
  apply();
  system.addEventListener('change', () => {
    try { preference = localStorage.getItem('etyloom.theme'); }
    catch (error) { console.warn('Theme preference is unavailable.', error); }
    apply();
  });
})();
