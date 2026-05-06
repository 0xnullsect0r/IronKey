/* ============================================================
   IronKey — main.js
   Handles: nav toggle, active nav, copy buttons,
            platform tabs, TOC highlight, tool search/filter
   ============================================================ */

'use strict';

/* ---- Nav mobile toggle ---- */
(function () {
  const toggle = document.getElementById('navToggle');
  const links  = document.getElementById('navLinks');
  if (!toggle || !links) return;
  toggle.addEventListener('click', () => {
    links.classList.toggle('open');
    toggle.setAttribute('aria-expanded', links.classList.contains('open'));
  });
  document.addEventListener('click', e => {
    if (!toggle.contains(e.target) && !links.contains(e.target)) {
      links.classList.remove('open');
    }
  });
})();

/* ---- Active nav link ---- */
(function () {
  const path = window.location.pathname;
  document.querySelectorAll('.nav-links a').forEach(a => {
    const href = a.getAttribute('href');
    if (!href) return;
    const normalized = href.replace(/^\.\.\//, '').replace(/index\.html$/, '').replace(/\/$/, '');
    const pathNorm   = path.replace(/index\.html$/, '').replace(/\/$/, '');
    if (normalized && pathNorm.endsWith(normalized)) {
      a.classList.add('active');
    }
  });
})();

/* ---- Copy code buttons ---- */
(function () {
  document.querySelectorAll('.code-header').forEach(header => {
    const btn = header.querySelector('.copy-btn');
    if (!btn) return;
    btn.addEventListener('click', () => {
      const pre = header.nextElementSibling;
      if (!pre) return;
      const text = pre.innerText || pre.textContent;
      navigator.clipboard.writeText(text.trim()).then(() => {
        btn.textContent = 'Copied!';
        btn.classList.add('copied');
        setTimeout(() => {
          btn.textContent = 'Copy';
          btn.classList.remove('copied');
        }, 2000);
      }).catch(() => {
        /* fallback: select text */
        const range = document.createRange();
        range.selectNode(pre);
        window.getSelection().removeAllRanges();
        window.getSelection().addRange(range);
      });
    });
  });
})();

/* ---- Platform tabs ---- */
(function () {
  document.querySelectorAll('.platform-tabs').forEach(container => {
    const buttons = container.querySelectorAll('.tab-btn');
    const panels  = container.querySelectorAll('.tab-panel');

    buttons.forEach(btn => {
      btn.addEventListener('click', () => {
        const target = btn.dataset.tab;
        buttons.forEach(b => b.classList.remove('active'));
        panels.forEach(p => p.classList.remove('active'));
        btn.classList.add('active');
        const panel = container.querySelector(`.tab-panel[data-tab="${target}"]`);
        if (panel) panel.classList.add('active');
      });
    });

    if (buttons.length) buttons[0].click();
  });
})();

/* ---- TOC active section highlight ---- */
(function () {
  const tocLinks = document.querySelectorAll('.toc a[href^="#"]');
  if (!tocLinks.length) return;

  const observer = new IntersectionObserver(entries => {
    entries.forEach(entry => {
      if (entry.isIntersecting) {
        const id = entry.target.getAttribute('id');
        tocLinks.forEach(a => {
          a.classList.toggle('active', a.getAttribute('href') === `#${id}`);
        });
      }
    });
  }, { rootMargin: '-15% 0px -75% 0px' });

  tocLinks.forEach(a => {
    const id = a.getAttribute('href').slice(1);
    const el = document.getElementById(id);
    if (el) observer.observe(el);
  });
})();

/* ---- Tools table: search, filter, count ---- */
(function () {
  const searchInput = document.getElementById('toolSearch');
  const filterSel   = document.getElementById('categoryFilter');
  const countEl     = document.getElementById('toolCount');
  const noResults   = document.getElementById('noResults');
  const tbody       = document.getElementById('toolTableBody');

  if (!searchInput || !tbody) return;

  function filter() {
    const q   = searchInput.value.toLowerCase().trim();
    const cat = filterSel ? filterSel.value.toLowerCase() : '';

    let visible = 0;
    tbody.querySelectorAll('tr').forEach(row => {
      const text    = row.textContent.toLowerCase();
      const rowCat  = (row.dataset.category || '').toLowerCase();
      const show    = (!q || text.includes(q)) && (!cat || rowCat === cat);
      row.style.display = show ? '' : 'none';
      if (show) visible++;
    });

    const total = tbody.querySelectorAll('tr').length;
    if (countEl) {
      countEl.textContent = (q || cat)
        ? `${visible} of ${total} tools`
        : `${total} tools`;
    }
    if (noResults) noResults.style.display = visible === 0 ? 'block' : 'none';
  }

  searchInput.addEventListener('input', filter);
  if (filterSel) filterSel.addEventListener('change', filter);
  filter();
})();

/* ---- Smooth scroll for anchor links ---- */
document.querySelectorAll('a[href^="#"]').forEach(a => {
  a.addEventListener('click', e => {
    const id = a.getAttribute('href').slice(1);
    const el = document.getElementById(id);
    if (el) {
      e.preventDefault();
      el.scrollIntoView({ behavior: 'smooth', block: 'start' });
    }
  });
});
