/* Sala Kiwi — JS mínimo: tema, pestañas y tablas ordenables.
   Sin dependencias. Todo funciona sin JS salvo estas mejoras. */
(function () {
  'use strict';

  /* ---------- Tema claro / oscuro ---------- */
  var app = document.querySelector('.app');
  var KEY = 'salakiwi-tema';
  try {
    var saved = localStorage.getItem(KEY);
    if (app && (saved === 'light' || saved === 'dark')) app.setAttribute('data-theme', saved);
  } catch (e) {}

  document.querySelectorAll('[data-theme-toggle]').forEach(function (btn) {
    btn.addEventListener('click', function () {
      if (!app) return;
      var next = app.getAttribute('data-theme') === 'light' ? 'dark' : 'light';
      app.setAttribute('data-theme', next);
      btn.setAttribute('aria-pressed', String(next === 'light'));
      try { localStorage.setItem(KEY, next); } catch (e) {}
    });
  });

  /* ---------- Pestañas (patrón ARIA tabs) ---------- */
  document.querySelectorAll('[role="tablist"]').forEach(function (list) {
    var tabs = Array.prototype.slice.call(list.querySelectorAll('[role="tab"]'));
    function select(tab) {
      tabs.forEach(function (t) {
        var on = t === tab;
        t.setAttribute('aria-selected', String(on));
        t.tabIndex = on ? 0 : -1;
        var panel = document.getElementById(t.getAttribute('aria-controls'));
        if (panel) panel.hidden = !on;
      });
    }
    tabs.forEach(function (tab, i) {
      tab.addEventListener('click', function () { select(tab); });
      tab.addEventListener('keydown', function (e) {
        var j = null;
        if (e.key === 'ArrowRight') j = (i + 1) % tabs.length;
        if (e.key === 'ArrowLeft') j = (i - 1 + tabs.length) % tabs.length;
        if (e.key === 'Home') j = 0;
        if (e.key === 'End') j = tabs.length - 1;
        if (j !== null) { e.preventDefault(); select(tabs[j]); tabs[j].focus(); }
      });
    });
  });

  /* ---------- Tablas ordenables ----------
     <table data-sortable> · <th><button class="sort"> · <td data-value="123"> */
  document.querySelectorAll('table[data-sortable]').forEach(function (table) {
    var ths = Array.prototype.slice.call(table.tHead.rows[0].cells);
    ths.forEach(function (th, col) {
      var btn = th.querySelector('.sort');
      if (!btn) return;
      btn.addEventListener('click', function () {
        var dir = th.getAttribute('aria-sort') === 'descending' ? 'ascending' : 'descending';
        ths.forEach(function (h) { h.removeAttribute('aria-sort'); });
        th.setAttribute('aria-sort', dir);
        var body = table.tBodies[0];
        var rows = Array.prototype.slice.call(body.rows);
        rows.sort(function (a, b) {
          var x = parseFloat(a.cells[col].getAttribute('data-value')) || 0;
          var y = parseFloat(b.cells[col].getAttribute('data-value')) || 0;
          return dir === 'descending' ? y - x : x - y;
        });
        rows.forEach(function (r, i) {
          body.appendChild(r);
          var rank = r.querySelector('.rank');
          if (rank) rank.textContent = String(i + 1);
        });
      });
    });
  });
})();
