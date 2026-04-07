const headerHTML = `
    <nav class="site-nav">
        <div class="nav-container">
            <a class="nav-logo" href="index.html">
                <picture>
                    <source srcset="assets/img/8b8tTrans-1.avif" type="image/avif">
                    <source srcset="assets/img/8b8tTrans-1.webp" type="image/webp">
                    <img src="assets/img/8b8tTrans-1.png" alt="8b8t">
                </picture>
            </a>

            <ul class="nav-links">
                <li><a href="index.html">Home</a></li>
                <li><a href="about.html">About</a></li>
                <li><a href="support.html">Support</a></li>
                <li><a href="stats.html">Stats</a></li>
                <li><a href="commands.html">Commands</a></li>
                <li><a href="lce.html">LCE</a></li>
                <li><a href="https://track.8b8t.me/">Track</a></li>
                <li><a href="https://shop.8b8t.me/">Shop</a></li>
                <li><a href="https://www.8b8t.me/vote">Vote</a></li>
                <li><a href="terms.html">Terms</a></li>
            </ul>

            <button class="hamburger" aria-label="Toggle menu">
                <svg class="hamburger-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <line x1="4" y1="6" x2="20" y2="6"></line>
                    <line x1="4" y1="12" x2="20" y2="12"></line>
                    <line x1="4" y1="18" x2="20" y2="18"></line>
                </svg>
                <svg class="close-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <line x1="18" y1="6" x2="6" y2="18"></line>
                    <line x1="6" y1="6" x2="18" y2="18"></line>
                </svg>
            </button>
        </div>

        <div class="mobile-menu">
            <ul class="mobile-nav-links">
                <li><a href="index.html">Home</a></li>
                <li><a href="about.html">About</a></li>
                <li><a href="support.html">Support</a></li>
                <li><a href="stats.html">Stats</a></li>
                <li><a href="commands.html">Commands</a></li>
                <li><a href="lce.html">LCE</a></li>
                <li><a href="https://track.8b8t.me/">Track</a></li>
                <li><a href="https://shop.8b8t.me/">Shop</a></li>
                <li><a href="https://www.8b8t.me/vote">Vote</a></li>
                <li><a href="terms.html">Terms</a></li>
            </ul>
        </div>
    </nav>
`;

const footerHTML = `
    <footer class="site-footer outer">
        <div class="inner">
            <div class="footer-grid">
                <div class="footer-column">
                    <h4>Community</h4>
                    <ul class="footer-links">
                        <li><a href="https://telegram.8b8t.me">Telegram</a></li>
                        <li><a href="https://videos.8b8t.me/">Videos</a></li>
                        <li><a href="https://votefor.8b8t.me/">Vote for Server</a></li>
                    </ul>
                </div>

                <div class="footer-column">
                    <h4>Server</h4>
                    <ul class="footer-links">
                        <li><a href="https://www.8b8t.me/how-to-join/">How to Join</a></li>
                        <li><a href="https://track.8b8t.me/">Track</a></li>
                        <li><a href="https://shop.8b8t.me/">Store</a></li>
                        <li><a href="https://track.8b8t.me/">Anarchy Server Track Site</a></li>
                    </ul>
                </div>

                <div class="footer-column">
                    <h4>Legal & Support</h4>
                    <ul class="footer-links">
                        <li><a href="https://www.8b8t.me/support">Help Center</a></li>
                        <li><a href="terms.html">Terms of Service</a></li>
                        <li><a href="lce.html">LCE</a></li>
                        <li><a href="privacy.html">Privacy Policy</a></li>
                        <li><a href="https://www.8b8t.me/support">Contact Us</a></li>
                    </ul>
                </div>
            </div>

            <div class="footer-bottom">
                <div class="footer-brand">
                    <picture>
                        <source srcset="assets/img/8b8tTrans-1.avif" type="image/avif">
                        <source srcset="assets/img/8b8tTrans-1.webp" type="image/webp">
                        <img class="footer-logo" src="assets/img/8b8tTrans-1.png" alt="8b8t">
                    </picture>
                    <span class="copyright">© 2026 8b8t Network</span>
                </div>

                <div class="social-links">
                    <a href="https://discord.8b8t.me"><i class="fa-brands fa-discord"></i></a>
                    <a href="https://www.youtube.com/@8b8tme"><i class="fa-brands fa-youtube"></i></a>
                    <a href="http://telegram.8b8t.me/"><i class="fa-brands fa-telegram"></i></a>
                    <a href="https://www.reddit.com/r/8b8t/"><i class="fa-brands fa-reddit"></i></a>
                </div>
            </div>

            <div class="gh-powered-by" style="margin-top: 20px; text-align: left;">
                <a href="https://xera.ca" target="_blank">A XERA INC. Brand</a>
            </div>
        </div>
    </footer>
`;

document.addEventListener('DOMContentLoaded', () => {
    const headerElement = document.getElementById('header-placeholder');
    if (headerElement) {
        headerElement.innerHTML = headerHTML;

        const currentPath = window.location.pathname.split('/').pop() || 'index.html';
        const navLinks = headerElement.querySelectorAll('.nav-links a, .mobile-nav-links a');
        navLinks.forEach(link => {
            const linkPath = link.getAttribute('href');
            if (linkPath === currentPath) {
                link.classList.add('nav-current');
            }
        });

        const hamburger = headerElement.querySelector('.hamburger');
        const mobileMenu = headerElement.querySelector('.mobile-menu');
        const nav = headerElement.querySelector('.site-nav');

        hamburger.addEventListener('click', () => {
            hamburger.classList.toggle('active');
            mobileMenu.classList.toggle('active');
            nav.classList.toggle('menu-open');
        });

        mobileMenu.querySelectorAll('a').forEach(link => {
            link.addEventListener('click', () => {
                hamburger.classList.remove('active');
                mobileMenu.classList.remove('active');
                nav.classList.remove('menu-open');
            });
        });
    }

    const footerElement = document.getElementById('footer-placeholder');
    if (footerElement) {
        footerElement.innerHTML = footerHTML;
    }
});
