var header = {}; //Empty object to store header data
var footer = {}; //Empty object to store footer data
switch (localStorage.getItem("preferredLanguage")) {
    case "es":
        header = {
            home: "Inicio",
            about: "Acerca de",
            support: "Soporte",
            stats: "Estadísticas",
            commands: "Comandos",
            lce: "LCE",
            track: "Sitio de Seguimiento",
            shop: "Tienda",
            vote: "Votar",
            terms: "Términos",
            switchLanguage: "Switch Language"
        };
        footer = {
            community: "Comunidad",
            telegram: "Telegram",
            videos: "Videos",
            voteForServer: "Votar por el servidor",
            server: "Servidor",
            howToJoin: "Cómo unirse",
            track: "Sitio de Seguimiento",
            shop: "Tienda",
            legalSupport: "Legal y Soporte",
            helpCenter: "Centro de ayuda",
            termsOfService: "Términos de servicio",
            lce: "LCE",
            privacyPolicy: "Política de privacidad",
            contactUs: "Contáctenos",
            operatedByXERA: "Operado por XERA INC"
        };
        break;

    default:
        header = {
            home: "Home",
            about: "About",
            support: "Support",
            stats: "Stats",
            commands: "Commands",
            lce: "LCE",
            track: "Track Site",
            shop: "Shop",
            vote: "Vote",
            terms: "Terms",
            switchLanguage: "Cambiar Idioma"
        };
        footer = {
            community: "Community",
            telegram: "Telegram",
            videos: "Videos",
            voteForServer: "Vote for Server",
            server: "Server",
            howToJoin: "How to Join",
            track: "Track Site",
            shop: "Shop",
            legalSupport: "Legal & Support",
            helpCenter: "Help Center",
            termsOfService: "Terms of Service",
            lce: "LCE",
            privacyPolicy: "Privacy Policy",
            contactUs: "Contact Us",
            operatedByXERA: "Operated by XERA INC"
        };
        break;
}
const headerHTML = `
    <nav class="site-nav">
        <div class="nav-container">
            <a class="nav-logo" href="index.html">
                <picture>
                    <source srcset="https://www.8b8t.me/assets/img/8b8tTrans-1.avif" type="image/avif">
                    <source srcset="https://www.8b8t.me/assets/img/8b8tTrans-1.webp" type="image/webp">
                    <img src="https://www.8b8t.me/assets/img/8b8tTrans-1.png" alt="8b8t">
                </picture>
            </a>

            <ul class="nav-links">
                <li><a href="index.html">${header.home}</a></li>
                <li><a href="about.html">${header.about}</a></li>
                <li><a href="support.html">${header.support}</a></li>
                <li><a href="stats.html">${header.stats}</a></li>
                <li><a href="commands.html">${header.commands}</a></li>
                <li><a href="lce.html">${header.lce}</a></li>
                <li><a href="https://track.8b8t.me/">${header.track}</a></li>
                <li><a href="https://shop.8b8t.me/">${header.shop}</a></li>
                <li><a href="https://www.8b8t.me/vote">${header.vote}</a></li>
                <li><a href="terms.html">${header.terms}</a></li>
                <li><button class="language-switcher" onclick="switchLanguage()">${header.switchLanguage}</button></li>
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
            <button class="language-switcher" onclick="switchLanguage()">${header.switchLanguage}</button>
                <li><a href="index.html">${header.home}</a></li>
                <li><a href="about.html">${header.about}</a></li>
                <li><a href="support.html">${header.support}</a></li>
                <li><a href="stats.html">${header.stats}</a></li>
                <li><a href="commands.html">${header.commands}</a></li>
                <li><a href="lce.html">${header.lce}</a></li>
                <li><a href="https://track.8b8t.me/">${header.track}</a></li>
                <li><a href="https://shop.8b8t.me/">${header.shop}</a></li>
                <li><a href="https://www.8b8t.me/vote">${header.vote}</a></li>
                <li><a href="terms.html">${header.terms}</a></li>
            </ul>
        </div>
    </nav>
`;

const footerHTML = `
    <footer class="site-footer outer">
        <div class="inner">
            <div class="footer-grid">
                <div class="footer-column">
                    <h4>${footer.community}</h4>
                    <ul class="footer-links">
                        <li><a href="https://telegram.8b8t.me">${footer.telegram}</a></li>
                        <li><a href="https://videos.8b8t.me/">${footer.videos}</a></li>
                        <li><a href="https://votefor.8b8t.me/">${footer.voteForServer}</a></li>
                    </ul>
                </div>

                <div class="footer-column">
                    <h4>${footer.server}</h4>
                    <ul class="footer-links">
                        <li><a href="https://www.8b8t.me/how-to-join/">${footer.howToJoin}</a></li>
                        <li><a href="https://track.8b8t.me/">${footer.track}</a></li>
                        <li><a href="https://shop.8b8t.me/">${footer.shop}</a></li>
                    </ul>
                </div>

                <div class="footer-column">
                    <h4>${footer.legalSupport}</h4>
                    <ul class="footer-links">
                        <li><a href="https://www.8b8t.me/support">${footer.helpCenter}</a></li>
                        <li><a href="terms.html">${footer.termsOfService}</a></li>
                        <li><a href="lce.html">${footer.lce}</a></li>
                        <li><a href="privacy.html">${footer.privacyPolicy}</a></li>
                        <li><a href="https://www.8b8t.me/support">${footer.contactUs}</a></li>
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
                <a href="https://xera.ca" target="_blank">${footer.operatedByXERA}.</a>
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
