// Custom Jaeger UI config: set browser tab title to include project name.
// UIConfig() is called by Jaeger's getJaegerUiConfig() at load time.
// The title override runs after DOM is ready, and MutationObserver keeps it
// stable against React's DocumentTitle re-renders.
function UIConfig() {
  var title = 'Jaeger UI ({{PROJECT_NAME}})';
  function applyTitle() {
    document.title = title;
    new MutationObserver(function() {
      if (document.title !== title) document.title = title;
    }).observe(document.querySelector('title'), { childList: true });
  }
  var titleEl = document.querySelector('title');
  if (titleEl) {
    applyTitle();
  } else {
    document.addEventListener('DOMContentLoaded', applyTitle);
  }
  return {};
}
