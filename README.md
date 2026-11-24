# BloxClient website

The website for [BloxClient](https://github.com/BudioustheIII/BloxClient), published at
https://budioustheiii.github.io/BloxClient/.

It's plain HTML, CSS and JavaScript with no build step. Every push to this branch deploys it to
GitHub Pages through `.github/workflows/deploy-website.yml`. All paths are relative, so the site
works under the repository's Pages subpath and when opened straight from disk.

- `index.html`, `styles.css`, `main.js`: the page
- `assets/screens/`: app screenshots, using made-up sample games and players
- `assets/download/`: the Windows installer
- `assets/mercury.js`: the app's gradients.sh Mercury background, prebuilt from the app source
  on the `main` branch. MIT licensed, see `assets/NOTICE-gradients-sh.md`
- `assets/fonts/`: Bricolage Grotesque and Hanken Grotesk, under the SIL Open Font License
