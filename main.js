const reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)");

/** Shows the still version of the gradient instead of the live one. */
function useStillGradient() {
  document.documentElement.classList.add("still-gradient");
}

/**
 * Times 60 frames a few times while the page settles. If the page can't hold
 * about 40 frames a second, it calls onSlow so the live gradient can stop.
 */
function watchFrameRate(onSlow) {
  const checks = [2500, 7000, 14000];
  const measure = () => {
    if (document.hidden) return;
    let frames = 0;
    let first = 0;
    requestAnimationFrame(function tick(now) {
      if (!first) first = now;
      if (++frames < 60) return requestAnimationFrame(tick);
      const fps = (59 * 1000) / (now - first);
      if (fps < 40) onSlow();
    });
  };
  checks.forEach((delay) => window.setTimeout(measure, delay));
}

/**
 * The app's own gradients.sh Mercury background, from assets/mercury.js. It
 * runs only in the hero and download sections, at half resolution, and pauses
 * when they're scrolled away. Everywhere else shows a still version.
 */
function setUpGradients() {
  const canvases = [...document.querySelectorAll("[data-mercury]")];
  const connection = navigator.connection;
  const lowPower =
    (connection && connection.saveData) ||
    (navigator.deviceMemory !== undefined && navigator.deviceMemory <= 2) ||
    (navigator.hardwareConcurrency !== undefined && navigator.hardwareConcurrency <= 2);
  if (lowPower || reduceMotion.matches) {
    useStillGradient();
    return;
  }

  const start = () => {
    if (!window.BloxMercury) {
      useStillGradient();
      return;
    }
    let mounted = [];
    try {
      mounted = canvases.map((canvas) => window.BloxMercury.mountMercury(canvas, { maxDpr: 0.5, maxFps: 30 }));
    } catch (error) {
      console.warn("The animated background didn't start, so the still version shows instead.", error);
      mounted.forEach((gradient) => gradient.unmount());
      useStillGradient();
      return;
    }
    watchFrameRate(() => {
      if (!mounted.length) return;
      mounted.forEach((gradient) => gradient.unmount());
      mounted = [];
      useStillGradient();
    });
  };
  if (window.BloxMercury) start();
  else document.getElementById("mercury-script").addEventListener("load", start, { once: true });
}

/** Splits the headline into words so they can fly in one after another. */
function splitHeadline() {
  const heading = document.querySelector("[data-split]");
  if (!heading) return;
  let index = 0;
  heading.querySelectorAll(".line").forEach((line) => {
    const words = line.textContent.trim().split(/\s+/);
    line.textContent = "";
    words.forEach((word, position) => {
      const span = document.createElement("span");
      span.className = "word";
      span.style.setProperty("--i", String(index++));
      span.textContent = word;
      line.append(span);
      if (position < words.length - 1) line.append(" ");
    });
  });
  heading.classList.add("split");
}

function setUpReveals() {
  document.querySelectorAll("[data-stagger]").forEach((group) => {
    [...group.querySelectorAll("[data-reveal]")].forEach((item, index) => {
      if (!item.style.getPropertyValue("--delay")) item.style.setProperty("--delay", `${index * 110}ms`);
    });
  });

  const items = document.querySelectorAll("[data-reveal]");
  if (reduceMotion.matches || !("IntersectionObserver" in window)) {
    items.forEach((item) => item.classList.add("is-in"));
    return;
  }
  const observer = new IntersectionObserver(
    (entries) => {
      entries.forEach((entry) => {
        if (!entry.isIntersecting) return;
        entry.target.classList.add("is-in");
        observer.unobserve(entry.target);
      });
    },
    { rootMargin: "0px 0px -8% 0px", threshold: 0.12 },
  );
  items.forEach((item) => observer.observe(item));
}

function setUpViewer() {
  const tabs = [...document.querySelectorAll('[role="tab"]')];
  const panel = document.getElementById("viewer-panel");
  const image = document.getElementById("viewer-image");
  const caption = document.getElementById("viewer-caption");
  const hotspots = document.getElementById("viewer-hotspots");
  const indicator = document.querySelector(".tab-indicator");
  let selected = 0;
  let request = 0;

  const sources = (name) => ({
    src: `assets/screens/${name}-1200.jpg`,
    srcset: `assets/screens/${name}-1200.jpg 1200w, assets/screens/${name}.jpg 2160w`,
  });

  function moveIndicator() {
    const tab = tabs[selected];
    indicator.style.width = `${tab.offsetWidth}px`;
    indicator.style.height = `${tab.offsetHeight}px`;
    indicator.style.transform = `translate(${tab.offsetLeft}px, ${tab.offsetTop}px)`;
  }

  async function show(index, moveFocus = false) {
    const tab = tabs[index];
    const current = ++request;
    selected = index;

    tabs.forEach((item, position) => {
      const isSelected = position === index;
      item.setAttribute("aria-selected", String(isSelected));
      item.tabIndex = isSelected ? 0 : -1;
    });
    moveIndicator();
    panel.setAttribute("aria-labelledby", tab.id);
    caption.textContent = tab.dataset.caption;
    tab.scrollIntoView({ block: "nearest", inline: "nearest" });
    if (moveFocus) tab.focus({ preventScroll: true });

    const { src, srcset } = sources(tab.dataset.screen);
    const next = new Image();
    const loaded = new Promise((resolve) => {
      next.onload = resolve;
      next.onerror = resolve;
      window.setTimeout(resolve, 2500);
    });
    next.sizes = image.sizes;
    next.srcset = srcset;
    next.src = src;
    image.classList.add("is-swapping");
    await loaded;
    if (current !== request) return;
    image.srcset = srcset;
    image.src = src;
    image.alt = tab.dataset.alt;
    image.classList.remove("is-swapping");
  }

  tabs.forEach((tab, index) => {
    tab.addEventListener("click", () => show(index));
    tab.addEventListener("keydown", (event) => {
      const moves = {
        ArrowRight: (index + 1) % tabs.length,
        ArrowLeft: (index - 1 + tabs.length) % tabs.length,
        Home: 0,
        End: tabs.length - 1,
      };
      if (!(event.key in moves)) return;
      event.preventDefault();
      show(moves[event.key], true);
    });

    // Invisible targets over the sidebar in the screenshot, for mouse users.
    const hotspot = document.createElement("button");
    hotspot.type = "button";
    hotspot.tabIndex = -1;
    hotspot.style.top = `${5.48 + index * 4.444}%`;
    hotspot.addEventListener("click", () => show(index));
    hotspots.append(hotspot);
  });

  // Place the indicator without sliding, then let later moves animate.
  moveIndicator();
  document.fonts?.ready.then(moveIndicator);
  window.addEventListener("resize", moveIndicator);
  requestAnimationFrame(() => indicator.classList.add("can-move"));
}

setUpGradients();
splitHeadline();
setUpViewer();
setUpReveals();
document.documentElement.classList.add("ready");
