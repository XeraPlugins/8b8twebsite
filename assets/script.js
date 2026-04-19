const galleryData = [
    { src: "https://www.8b8t.me/assets/players/2021-07-03-Imperials-Base.avif", author: "Imperials" },
    { src: "https://www.8b8t.me/assets/players/2022-07-01-Harsh-Prakhar.avif", author: "Harsh Prakhar" },
    { src: "https://www.8b8t.me/assets/players/2022-10-29-Suns-World-Base.avif", author: "Suns" },
    { src: "https://www.8b8t.me/assets/players/2022-11-19-DPS.avif", author: "DPS" },
    { src: "https://www.8b8t.me/assets/players/2022-11-29-2-KurtzMC.webp", author: "KurtzMC" },
    { src: "https://www.8b8t.me/assets/players/2022-11-29-KurtzMC.webp", author: "KurtzMC" },
    { src: "https://www.8b8t.me/assets/players/2023-08-03-TheTroll2001-Megabase.avif", author: "TheTroll2001" },
    { src: "https://www.8b8t.me/assets/players/2024-02-25-SSamu-Invasion.avif", author: "SSamu" },
    { src: "https://www.8b8t.me/assets/players/2024-02-25-SSamu-Invasion-2.avif", author: "SSamu" },
    { src: "https://www.8b8t.me/assets/players/2024-04-23-DPS-End-Watercube.avif", author: "End Watercube (DPS)" },
    { src: "https://www.8b8t.me/assets/players/2025-01-03-Griefcon.webp", author: "Griefcon 4 (DPS)" },
    { src: "https://www.8b8t.me/assets/players/2025-03-26-Pigy-Mega-Base-Grief.avif", author: "Pigy" },
    { src: "https://www.8b8t.me/assets/players/2025-07-09-Operation-Overflow.avif", author: "Operation Overflow (DPS)" },
    { src: "https://www.8b8t.me/assets/players/2025-07-15-DPS-Hellstorm.avif", author: "Operation Hellstorm (DPS)" },
    { src: "https://www.8b8t.me/assets/players/2025-08-23-Focoloa.webp", author: "Focoloa" },
    { src: "https://www.8b8t.me/assets/players/2025-09-03-MindComplexity.webp", author: "MindComplexity" },
    { src: "https://www.8b8t.me/assets/players/2025-09-03-Obsidian-Roof.avif", author: "Team Avolution" },
    { src: "https://www.8b8t.me/assets/players/2025-09-07-MindComplexity.avif", author: "MindComplexity" },
    { src: "https://www.8b8t.me/assets/players/2025-09-07-MindComplexity-Spawn-Logo.avif", author: "MindComplexity" },
    { src: "https://www.8b8t.me/assets/players/2025-09-07-T7-Statue-MindComplexity.avif", author: "MindComplexity" },
    { src: "https://www.8b8t.me/assets/players/2025-09-13-Golden-Gooners.webp", author: "Golden Gooners" },
    { src: "https://www.8b8t.me/assets/players/2025-10-17-thetroll2001.webp", author: "TheTroll2001" },
    { src: "https://www.8b8t.me/assets/players/2025-10-30-MrDarkEye.webp", author: "MrDarkEye" },
    { src: "https://www.8b8t.me/assets/players/2025-11-10-KurtzMC.webp", author: "KurtzMC" },
    { src: "https://www.8b8t.me/assets/players/2025-11-12-Test-Server.webp", author: "TBD" },
    { src: "https://www.8b8t.me/assets/players/2025-11-25.webp", author: "TBD" },
    { src: "https://www.8b8t.me/assets/players/2025-11-26-Lev.webp", author: "Lev" },
    { src: "https://www.8b8t.me/assets/players/2025-11-29-Mori.webp", author: "Mori" },
    { src: "https://www.8b8t.me/assets/players/2025-11-Malbacoo.webp", author: "Malbacoo" },
    { src: "https://www.8b8t.me/assets/players/2025-12-07-Gosankyo.webp", author: "Gosankyo" },
    { src: "https://www.8b8t.me/assets/players/2025-12-08-Darrplayer.webp", author: "Darrplayer" },
    { src: "https://www.8b8t.me/assets/players/2025-12-21-Gosankyo.webp", author: "Gosankyo" },
    { src: "https://www.8b8t.me/assets/players/2025-12-28-Zeentri.webp", author: "Zeentri" },
    { src: "https://www.8b8t.me/assets/players/2025-12-31-Team-Avolution.avif", author: "Team Avolution" },
    { src: "https://www.8b8t.me/assets/players/KurtzMC.webp", author: "KurtzMC" },
    { src: "https://www.8b8t.me/assets/players/Sin-Titulo2-TBD.webp", author: "TBD" },
    { src: "https://www.8b8t.me/assets/players/2023-10-30_19.33.08-tobias.webp", author: "Tobias" },
    { src: "https://www.8b8t.me/assets/players/2023-12-16_20.27.22-tobias.webp", author: "Tobias" },
    { src: "https://www.8b8t.me/assets/players/2024-01-31_19.53.29-tobias.webp", author: "Tobias" },
    { src: "https://www.8b8t.me/assets/players/2024-11-23_18.06.10-tobias.webp", author: "Tobias" },
    { src: "https://www.8b8t.me/assets/players/2025-01-04_21.36.46-tobias.webp", author: "Tobias" },
    { src: "https://www.8b8t.me/assets/players/2026-01-10-Avolution.avif", author: "Team Avolution" },
    { src: "https://www.8b8t.me/assets/players/2026-01-11-T7.avif", author: "T7" },
    { src: "https://www.8b8t.me/assets/players/2026-01-11-MindComplexity.avif", author: "MindComplexity" },
    { src: "https://www.8b8t.me/assets/players/2025-07-09-Overflow.avif", author: "DPS Project Overflow" }

];

const features = [
    {
        tag: "World Generation",
        title: "Secure Seed Technology",
        desc: "The world is unique and cannot be cracked. We feature custom world generation to ensure fair exploration for everyone.",
        img: "https://www.8b8t.me/assets/img/secure_seed.webp"
    },
    {
        tag: "Fair Play",
        title: "Custom Anti-Cheat",
        desc: "We have our own anti-cheat designed specifically for anarchy gameplay. No generic, cancer ACs like NCP or Grim that ruin the experience.",
        img: "https://www.8b8t.me/assets/img/custom_ac.avif"
    },
    {
        tag: "Combat Evolved",
        title: "Spear PvP Meta",
        desc: "We have multitasking and Spear PVP meta. While Crystal PvP is vanilla, spears offer a superior air advantage when paired with elytrafly.",
        img: "https://www.8b8t.me/assets/img/spear.avif"
    },
    {
        tag: "Gameplay",
        title: "20+ Custom Commands",
        desc: "We have dozens of commands that improve the quality of life drastically. Designed to make the server less tedious.",
        img: "https://www.8b8t.me/assets/img/gameplay.webp"
    },
    {
        tag: "Updates",
        title: "Latest Minecraft Version",
        desc: "We are always up to date. We support 1.21+ (Recommended 1.21.8) so you can enjoy the newest blocks and features.",
        img: "https://www.8b8t.me/assets/img/latest.webp"
    }
];

let currentIndex = 0;
let autoCycleInterval;
const featuresArea = document.getElementById('features-area');

function updateFeatureUI(index) {
    const buttons = document.querySelectorAll('.feature-btn');
    if (!buttons.length) return;

    buttons.forEach(btn => {
        btn.classList.remove('active');
        void btn.offsetWidth;
    });
    buttons[index].classList.add('active');

    const f = features[index];
    const tagElem = document.getElementById('feature-tag');
    const titleElem = document.getElementById('feature-title');
    const descElem = document.getElementById('feature-desc');

    if (tagElem) tagElem.textContent = f.tag;
    if (titleElem) titleElem.textContent = f.title;
    if (descElem) descElem.textContent = f.desc;

    const img = document.getElementById('feature-img');
    const avifSource = document.getElementById('feature-avif');
    const webpSource = document.getElementById('feature-webp');

    if (img) {
        img.style.opacity = '0';
        setTimeout(() => {
            const filename = f.img.split('/').pop().replace(/\.(webp|avif|png|jpg)$/, '');
            const optimizedSrc = `assets/img/optimized/${filename}.webp`;

            if (avifSource) avifSource.srcset = '';
            if (webpSource) webpSource.srcset = optimizedSrc;

            img.src = optimizedSrc;
            img.style.opacity = '1';
        }, 300);
    }
}

function nextFeature() {
    currentIndex = (currentIndex + 1) % features.length;
    updateFeatureUI(currentIndex);
}

function manualFeature(index) {
    if (currentIndex === index) return;
    currentIndex = index;
    updateFeatureUI(currentIndex);
    stopAutoCycle();
    startAutoCycle();
}

function startAutoCycle() {
    if (autoCycleInterval) clearInterval(autoCycleInterval);
    autoCycleInterval = setInterval(nextFeature, 4000);
}

function stopAutoCycle() {
    if (autoCycleInterval) {
        clearInterval(autoCycleInterval);
        autoCycleInterval = null;
    }
}

if (featuresArea) {
    currentIndex = 0;
    updateFeatureUI(0);
    startAutoCycle();

    featuresArea.addEventListener('mouseenter', () => {
        stopAutoCycle();
    });

    featuresArea.addEventListener('mouseleave', () => {
        const activeBtn = document.querySelector('.feature-btn.active');
        if (activeBtn) {
            activeBtn.classList.remove('active');
            void activeBtn.offsetWidth;
            activeBtn.classList.add('active');
        }
        startAutoCycle();
    });
}

const hotswapGrid = document.getElementById('hotswap-grid');
const paginationContainer = document.getElementById('pagination');
const MAX_VISIBLE = 12;
let currentPage = 1;
let masterShuffledList = [];

function shuffleArray(array) {
    const newArr = [...array];
    for (let i = newArr.length - 1; i > 0; i--) {
        const j = Math.floor(Math.random() * (i + 1));
        [newArr[i], newArr[j]] = [newArr[j], newArr[i]];
    }
    return newArr;
}

function extractDate(src) {
    const match = src.match(/(\d{4}-\d{2}-\d{2})/);
    if (match) {
        const date = new Date(match[1]);
        return date.toLocaleDateString('en-US', { year: 'numeric', month: 'long', day: 'numeric' });
    }
    return "Unknown Date";
}

function extractFileType(src) {
    const ext = src.split('.').pop().toUpperCase();
    return ext === 'WEBP' ? 'WebP' : ext;
}

function createGalleryItem(item, slotIndex) {
    const dateStr = extractDate(item.src);
    const fileType = extractFileType(item.src);
    const filename = item.src.split('/').pop().replace(/\.(webp|avif|png|jpg)$/, '');
    const div = document.createElement('div');
    div.className = 'gallery-item';
    div.id = `gallery-slot-${slotIndex}`;
    div.setAttribute('data-src', item.src);
    div.setAttribute('data-author', item.author);
    div.setAttribute('data-date', dateStr);

    const thumbSrc = `assets/players/thumbs/${filename}.webp`;

    div.innerHTML = `
        <div class="gallery-image-container">
            <picture>
                <img src="${thumbSrc}" class="gallery-image" alt="Build by ${item.author}" loading="lazy">
            </picture>
            <div class="gallery-overlay">
                <i class="fa-solid fa-magnifying-glass-plus" style="color: #fff; font-size: 28px;"></i>
            </div>
        </div>
        <div class="gallery-card-content">
            <div class="gallery-card-info">
                <span class="gallery-card-author">${item.author}</span>
                <div class="gallery-card-date">
                    <i class="fa-solid fa-calendar-days"></i> ${dateStr}
                </div>
            </div>
            <div class="gallery-card-type">${fileType}</div>
        </div>
    `;
    return div;
}

function renderPagination() {
    if (!paginationContainer) return;
    const totalPages = Math.ceil(masterShuffledList.length / MAX_VISIBLE);
    paginationContainer.innerHTML = '';

    for (let i = 1; i <= totalPages; i++) {
        const btn = document.createElement('div');
        btn.className = `pagination-item ${i === currentPage ? 'active' : ''}`;
        btn.textContent = i;
        btn.onclick = () => goToPage(i);
        paginationContainer.appendChild(btn);
    }
}

function goToPage(page) {
    if (page === currentPage) return;
    currentPage = page;
    renderPagination();
    renderGalleryPage();
}

function renderGalleryPage() {
    if (!hotswapGrid) return;
    hotswapGrid.style.opacity = '0';
    hotswapGrid.style.transform = 'translateY(15px)';
    hotswapGrid.style.transition = 'all 0.4s cubic-bezier(0.165, 0.84, 0.44, 1)';

    setTimeout(() => {
        hotswapGrid.innerHTML = '';

        const startIndex = (currentPage - 1) * MAX_VISIBLE;
        const pageItems = masterShuffledList.slice(startIndex, startIndex + MAX_VISIBLE);

        pageItems.forEach((item, i) => {
            hotswapGrid.appendChild(createGalleryItem(item, i));
        });

        hotswapGrid.style.opacity = '1';
        hotswapGrid.style.transform = 'translateY(0)';
    }, 350);
}

function initGallery() {
    if (!hotswapGrid) return;
    masterShuffledList = shuffleArray(galleryData);
    renderPagination();
    renderGalleryPage();
}

initGallery();


function openLightbox(slotIndex) {
    const slotElement = document.getElementById(`gallery-slot-${slotIndex}`);
    if (!slotElement) return;

    const src = slotElement.getAttribute('data-src');
    const author = slotElement.getAttribute('data-author');
    const date = slotElement.getAttribute('data-date');

    const lb = document.getElementById('lightbox');
    const lbImg = document.getElementById('lb-img');
    const lbAvif = document.getElementById('lb-avif');
    const lbWebp = document.getElementById('lb-webp');
    const lbAuthor = document.getElementById('lb-author');

    if (lbImg) {
        const base = src.substring(0, src.lastIndexOf('.'));
        const ext = src.split('.').pop().toLowerCase();
        if (lbAvif) lbAvif.srcset = (ext === 'avif') ? base + '.avif' : '';
        if (lbWebp) lbWebp.srcset = base + '.webp';
        lbImg.src = src;
    }
    if (lbAuthor) {
        lbAuthor.innerHTML = `${author} <span style="margin-left: 15px; font-weight: 400; font-size: 14px; color: #86868b;">${date}</span>`;
    }

    if (lb) lb.classList.add('active');
    document.body.style.overflow = 'hidden';
}

function closeLightbox() {
    const lb = document.getElementById('lightbox');
    if (!lb) return;
    const lbContent = lb.querySelector('.lightbox-content');
    lb.classList.remove('active');
    if (lbContent) lbContent.classList.remove('zoomed');
    document.body.style.overflow = '';
}

const lbContent = document.querySelector('.lightbox-content');
if (lbContent) {
    lbContent.addEventListener('click', function (e) {
        e.stopPropagation();
        const img = this.querySelector('img');
        if (!this.classList.contains('zoomed')) {
            const rect = this.getBoundingClientRect();
            const x = ((e.clientX - rect.left) / rect.width) * 100;
            const y = ((e.clientY - rect.top) / rect.height) * 100;
            img.style.transformOrigin = `${x}% ${y}%`;
            this.classList.add('zoomed');
        } else {
            this.classList.remove('zoomed');
            setTimeout(() => {
                img.style.transformOrigin = 'center center';
            }, 400);
        }
    });
}

if (hotswapGrid) {
    hotswapGrid.addEventListener('click', (e) => {
        const item = e.target.closest('.gallery-item');
        if (item) {
            const slotIndex = parseInt(item.id.replace('gallery-slot-', ''));
            openLightbox(slotIndex);
        }
    });
}

function copyToClipboard(text) {
    navigator.clipboard.writeText(text);
    alert("Copied IP: " + text);
}

function updatePlayerCount() {
    const counter = document.getElementById('player-count');
    if (!counter) return;

    fetch('https://api.mcsrvstat.us/2/8b8t.me')
        .then(res => res.json())
        .then(data => {
            if (data.online && data.players) {
                counter.textContent = data.players.online;
            } else {
                counter.textContent = '...';
            }
        })
        .catch(() => {
            counter.textContent = '...';
        });
}

updatePlayerCount();

document.addEventListener('DOMContentLoaded', () => {
    const faqItems = document.querySelectorAll('.faq-item');
    if (faqItems.length > 0) {
        faqItems.forEach(item => {
            const question = item.querySelector('.faq-question');
            if (question) {
                question.addEventListener('click', () => {
                    const isActive = item.classList.contains('active');

                    faqItems.forEach(otherItem => otherItem.classList.remove('active'));

                    if (!isActive) {
                        item.classList.add('active');
                    }
                });
            }
        });
    }
});

function initCommunitySwitcher() {
    const switcher = document.getElementById('community-switcher');
    if (!switcher) return;

    let flipped = false;
    setInterval(() => {
        flipped = !flipped;
        if (flipped) {
            switcher.classList.add('flipped');
        } else {
            switcher.classList.remove('flipped');
        }
    }, 6000);
}

function initFAQV3() {
    const faqItems = document.querySelectorAll('.faq-v3-item');
    if (faqItems.length === 0) return;

    faqItems.forEach(item => {
        const header = item.querySelector('.faq-v3-header');
        header.addEventListener('click', () => {
            const isActive = item.classList.contains('active');

            faqItems.forEach(other => other.classList.remove('active'));

            if (!isActive) {
                item.classList.add('active');
            }
        });
    });
}

function initSmoothScroll() {
    const links = document.querySelectorAll('.scroll-link');
    links.forEach(link => {
        link.addEventListener('click', (e) => {
            e.preventDefault();
            const targetId = link.getAttribute('href');
            const targetElement = document.querySelector(targetId);
            if (targetElement) {
                targetElement.scrollIntoView({ behavior: 'smooth' });
            }
        });
    });
}

document.addEventListener('DOMContentLoaded', () => {
    initCommunitySwitcher();
    initFAQV3();
    initSmoothScroll();
});
