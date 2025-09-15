// Global state
const state = {
    currentUrl: '',
    currentModel: '',
    availableModels: [],
    chatVisible: true,
    tabs: [],
    activeTabId: '',
    ollamaEnabled: false,
    ollamaModelsCache: [],
    slashVisible: false,
    slashIndex: -1
};

// DOM Elements
let urlInput, browserViewContainer, welcomeScreen, chatMessages, messageInput, 
    sendButton, statusIndicator, currentUrlSpan, modelSelect, chatPanel,
    chatToggle, modeOnlineBtn, modeLocalBtn;
let settingsBtn, settingsModal, settingsClose, settingsSave, ollamaUrlInput, enableOllamaCheckbox, ollamaSettings;
let popularSitesContainer, popularEditBtn, popularModal, popularCloseBtn, popularList, popularIdInput, popularTitleInput, popularUrlInput, popularColorInput, popularIconInput, popularResetBtn;
let popularAutoSaveTimer = null;
let tabsContainer, newTabBtn;
let slashBox;

// Tauri API
let tauriReady = false;
let tauriInvoke = null;
let appWindow = null;
let panelResizeObserver = null;
let roDebounceTimer = null;
let modelsRequestToken = 0; // race-condition guard for model loading

// Wait for Tauri to be ready
function waitForTauri() {
    return new Promise((resolve) => {
        const check = () => {
            if (window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.window) {
                console.log('Tauri API yüklendi!');
                tauriInvoke = window.__TAURI__.core.invoke;
                appWindow = window.__TAURI__.window.appWindow;
                tauriReady = true;
                resolve();
                return true;
            }
            return false;
        };

        if (check()) return;

        let attempts = 0;
        const interval = setInterval(() => {
            attempts++;
            if (check()) {
                clearInterval(interval);
            } else if (attempts > 20) {
                clearInterval(interval);
                resolve(); // Resolve anyway, tauriReady will be false
            }
        }, 500);
    });
}

function setupResizeObserver() {
    const panelEl = document.getElementById('browser-panel');
    if (panelEl && 'ResizeObserver' in window) {
        if (panelResizeObserver) {
            panelResizeObserver.disconnect();
        }
        panelResizeObserver = new ResizeObserver(() => {
            // debounce rapid resize events
            clearTimeout(roDebounceTimer);
            roDebounceTimer = setTimeout(() => {
                updateBrowserViewPositionAndSize();
            }, 50);
        });
        panelResizeObserver.observe(panelEl);
    }
}

// Initialize app
document.addEventListener('DOMContentLoaded', async () => {
    console.log('DOM yüklendi, app başlatılıyor...');
    
    initializeElements();
    
    // Show welcome screen initially
    showWelcomeScreen();
    
    // Wait for Tauri API and then set up listeners that depend on it
    await waitForTauri();
    
    setupEventListeners(); // Always setup listeners
    setupResizeObserver();

    if (tauriReady) {
        console.log('Tauri API hazır.');
        
        // Başlangıç modu: online/local
        const savedMode = localStorage.getItem('ai_mode');
        if (savedMode === 'local') setMode('local'); else setMode('online');
        
        createNewTab();
        // Load popular sites into homepage
        loadPopularSites();
        // Listen menu events to open settings
        try {
            const tauriEvent = window.__TAURI__ && window.__TAURI__.event;
            if (tauriEvent && typeof tauriEvent.listen === 'function') {
                await tauriEvent.listen('open-settings', () => {
                    openSettings();
                });
            }
        } catch (e) {
            console.error('Menu event dinlenemedi:', e);
        }
    } else {
        console.error('Tauri API yüklenemedi, fallback moda geçiliyor');
        if (modelSelect) {
            modelSelect.innerHTML = '<option value="">Tauri API yüklenemedi</option>';
        }
        updateChatStatus('Tauri bağlantısı yok', 'error');
    }
    
    console.log('Local Browser başlatıldı');
});

function initializeElements() {
    urlInput = document.getElementById('url-input');
    browserViewContainer = document.getElementById('browser-view-container');
    welcomeScreen = document.getElementById('welcome-screen');
    chatMessages = document.getElementById('chat-messages');
    messageInput = document.getElementById('message-input');
    sendButton = document.getElementById('send-btn');
    statusIndicator = document.getElementById('status-indicator');
    currentUrlSpan = document.getElementById('current-url');
    modelSelect = document.getElementById('model-select');
    chatPanel = document.getElementById('chat-panel');
    chatToggle = document.getElementById('chat-toggle');
    slashBox = document.getElementById('slash-suggestions');
    // Settings
    settingsBtn = document.getElementById('settings-btn');
    settingsModal = document.getElementById('settings-modal');
    settingsClose = document.getElementById('settings-close');
    settingsSave = document.getElementById('settings-save');
    ollamaUrlInput = document.getElementById('ollama-url');
    enableOllamaCheckbox = document.getElementById('enable-ollama');
    ollamaSettings = document.getElementById('ollama-settings');
    // Popular sites
    popularSitesContainer = document.getElementById('popular-sites');
    popularEditBtn = document.getElementById('popular-edit-btn');
    popularModal = document.getElementById('popular-modal');
    popularCloseBtn = document.getElementById('popular-close');
    popularList = document.getElementById('popular-list');
    popularIdInput = document.getElementById('popular-id');
    popularTitleInput = document.getElementById('popular-title');
    popularUrlInput = document.getElementById('popular-url');
    popularColorInput = document.getElementById('popular-color');
    popularIconInput = document.getElementById('popular-icon');
    popularResetBtn = document.getElementById('popular-reset');
    // Mode toggle
    modeOnlineBtn = document.getElementById('mode-online');
    modeLocalBtn = document.getElementById('mode-local');
    // Tabs
    tabsContainer = document.getElementById('tabs-container');
    newTabBtn = document.getElementById('new-tab-btn');
}

function setupEventListeners() {
    // URL input events
    urlInput.addEventListener('keypress', (e) => {
        if (e.key === 'Enter') {
            handleUrlSubmit();
        }
    });
    
    document.getElementById('go-btn').addEventListener('click', handleUrlSubmit);
    
    // Navigation buttons
    document.getElementById('back-btn').addEventListener('click', () => {
        if (tauriInvoke && state.activeTabId) {
            tauriInvoke('navigate_back', { tabId: state.activeTabId }).catch(err => console.error(err));
        }
    });
    
    document.getElementById('forward-btn').addEventListener('click', () => {
        if (tauriInvoke && state.activeTabId) {
            tauriInvoke('navigate_forward', { tabId: state.activeTabId }).catch(err => console.error(err));
        }
    });
    
    document.getElementById('refresh-btn').addEventListener('click', () => {
        if (tauriInvoke && state.activeTabId) {
            tauriInvoke('reload_page', { tabId: state.activeTabId }).catch(err => console.error(err));
        }
    });

    document.getElementById('home-btn').addEventListener('click', goToHome);
    
    // Homepage search functionality
    const homepageSearch = document.getElementById('homepage-search');
    const homepageSearchBtn = document.getElementById('homepage-search-btn');
    
    if (homepageSearch && homepageSearchBtn) {
        const handleHomepageSearch = () => {
            const query = homepageSearch.value.trim();
            if (!query) return;
            
            // Check if it's a URL or search term
            if (query.includes('.') && !query.includes(' ')) {
                // Looks like a URL
                loadUrl(query);
            } else {
                // Search on Google
                loadUrl(`https://www.google.com/search?q=${encodeURIComponent(query)}`);
            }
        };
        
        homepageSearch.addEventListener('keypress', (e) => {
            if (e.key === 'Enter') {
                handleHomepageSearch();
            }
        });
        
        homepageSearchBtn.addEventListener('click', handleHomepageSearch);
    }
    
    // Chat events
    // Keydown: handle slash suggestions navigation first
    messageInput.addEventListener('keydown', (e) => {
        if (state.slashVisible) {
            if (e.key === 'ArrowDown' || e.key === 'Tab') {
                e.preventDefault();
                moveSlashSelection(1);
                return;
            }
            if (e.key === 'ArrowUp') {
                e.preventDefault();
                moveSlashSelection(-1);
                return;
            }
            if (e.key === 'Enter') {
                // Accept selection instead of submit
                const selected = getSlashSelectedItem();
                if (selected) {
                    e.preventDefault();
                    insertSlashCommand(selected.dataset.cmd);
                    return;
                }
            }
            if (e.key === 'Escape') {
                hideSlashSuggestions();
                return;
            }
        }
        // Submit with Enter
        if (e.key === 'Enter' && !e.shiftKey) {
            e.preventDefault();
            handleChatSubmit();
        }
    });
    
    sendButton.addEventListener('click', handleChatSubmit);
    
    // Chat controls
    document.getElementById('clear-chat').addEventListener('click', clearChat);
    document.getElementById('minimize-chat').addEventListener('click', toggleChat);
    chatToggle.addEventListener('click', toggleChat);
    
    // Model selection - varsayılan olarak OpenRouter, Ollama etkinse model seçici göster
    if (modelSelect) {
        state.currentModel = 'openrouter:google/gemini-2.0-flash-exp:free';

        // Kullanıcı model seçtiğinde güncelle
        modelSelect.addEventListener('change', (e) => {
            const value = e.target.value;
            state.currentModel = value;
            const label = modelSelect.options[modelSelect.selectedIndex]?.textContent || value;
            updateChatStatus(`Model: ${label}`);
        });
    }
    
    // Auto-resize textarea
    messageInput.addEventListener('input', () => {
        messageInput.style.height = 'auto';
        messageInput.style.height = Math.min(messageInput.scrollHeight, 100) + 'px';
        handleSlashInput();
    });

    // Add window resize listener
    window.addEventListener('resize', updateBrowserViewPositionAndSize);

    // Settings modal events
    if (settingsBtn && settingsModal) {
        settingsBtn.addEventListener('click', openSettings);
    }
    if (settingsClose) {
        settingsClose.addEventListener('click', closeSettings);
    }
    if (settingsSave) {
        settingsSave.addEventListener('click', saveSettings);
    }
    // Popular sites modal events
    if (popularEditBtn) popularEditBtn.addEventListener('click', openPopularModal);
    if (popularCloseBtn) popularCloseBtn.addEventListener('click', closePopularModal);
    if (popularResetBtn) popularResetBtn.addEventListener('click', resetPopularForm);
    // Auto-save on input changes
    [popularTitleInput, popularUrlInput, popularColorInput, popularIconInput].forEach(inp => {
        if (inp) inp.addEventListener('input', schedulePopularAutoSave);
    });
    if (enableOllamaCheckbox) {
        enableOllamaCheckbox.addEventListener('change', toggleOllama);
    }
    if (modeOnlineBtn && modeLocalBtn) {
        modeOnlineBtn.addEventListener('click', () => setMode('online'));
        modeLocalBtn.addEventListener('click', () => setMode('local'));
    }

    // Tabs events
    if (newTabBtn) newTabBtn.addEventListener('click', () => createNewTab());
    document.addEventListener('keydown', (e) => {
        if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 't') {
            e.preventDefault();
            createNewTab();
        }
        // Ctrl+W: Close active tab
        if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'w') {
            e.preventDefault();
            if (state.activeTabId) {
                closeTab(state.activeTabId);
            }
        }
    });

    // Streaming: listen via Tauri event API (reliable for custom events)
    const tauriEvent = window.__TAURI__ && window.__TAURI__.event;
    if (tauriEvent && typeof tauriEvent.listen === 'function') {
        let currentBotMessageDiv = null;
        let fullResponse = '';

        // İçerik kaynağı bilgisi: hem konsola logla hem de sohbete kısa sistem mesajı ekle
        tauriEvent.listen('content-source', (event) => {
            const p = event && event.payload ? event.payload : null;
            if (!p) return;
            console.group('[CONTENT SOURCE]');
            console.log('mode:', p.mode);
            console.log('url:', p.url);
            console.log('source:', p.source); // youtube_light | firecrawl_scrape | firecrawl_crawl | http_fallback | cache_unknown
            console.log('from_cache:', p.from_cache);
            console.log('length:', p.length);
            console.log('preview:', p.preview);
            console.groupEnd();
        });

        tauriEvent.listen('ollama-stream', (event) => {
            const payload = event && event.payload ? event.payload : null;

            if (!payload) return;

            // Ensure message bubble exists
            if (!currentBotMessageDiv) {
                currentBotMessageDiv = addChatMessage('bot', '');
            }

            if (payload.response) {
                fullResponse += payload.response;
                if (window.marked) {
                    currentBotMessageDiv.innerHTML = marked.parse(fullResponse);
                } else {
                    currentBotMessageDiv.textContent = fullResponse;
                }
                chatMessages.scrollTop = chatMessages.scrollHeight;
            }

            if (payload.done) {
                // If model produced no tokens at all, keep empty bubble to show completion
                if (!fullResponse) {
                    currentBotMessageDiv.textContent = '(cevap üretilemedi)';
                }
                currentBotMessageDiv = null;
                fullResponse = '';
                sendButton.disabled = false;
                setChatLoading(false);
            }
        });

        // OpenRouter streaming
        tauriEvent.listen('openrouter-stream', (event) => {
            const payload = event && event.payload ? event.payload : null;
            if (!payload) return;
            if (!currentBotMessageDiv) {
                currentBotMessageDiv = addChatMessage('bot', '');
            }
            if (payload.response) {
                fullResponse += payload.response;
                if (window.marked) {
                    currentBotMessageDiv.innerHTML = marked.parse(fullResponse);
                } else {
                    currentBotMessageDiv.textContent = fullResponse;
                }
                chatMessages.scrollTop = chatMessages.scrollHeight;
            }
            if (payload.done) {
                if (!fullResponse) { currentBotMessageDiv.textContent = '(cevap üretilemedi)'; }
                currentBotMessageDiv = null;
                fullResponse = '';
                sendButton.disabled = false;
                setChatLoading(false);
            }
        });

        // OpenRouter fallback model değişimleri
        tauriEvent.listen('openrouter-model-fallback', (event) => {
            const p = event && event.payload ? event.payload : null;
            if (p && p.to) {
                console.warn('[OpenRouter Fallback] Yeni model:', p.to);
            } else {
                console.warn('[OpenRouter Fallback] Model değişimi');
            }
        });
    }

    // Listen for page load events from Tauri
    if (appWindow) {
        appWindow.listen('tauri://page-load', () => {
            console.log('Tauri page load event received');
            updateStatus('ready');
            updateBrowserViewPositionAndSize();
        });
    }

    // Listen for page title and favicon updates
    if (tauriEvent && typeof tauriEvent.listen === 'function') {
        tauriEvent.listen('page-title-changed', (event) => {
            const { tabId, title } = event.payload || {};
            if (tabId && title) {
                const tab = state.tabs.find(t => t.id === tabId);
                if (tab) {
                    tab.title = title;
                    renderTabs();
                }
            }
        });

        tauriEvent.listen('page-favicon-changed', (event) => {
            const { tabId, favicon } = event.payload || {};
            if (tabId && favicon) {
                const tab = state.tabs.find(t => t.id === tabId);
                if (tab) {
                    tab.favicon = favicon;
                    renderTabs();
                }
            }
        });

        // Webview içi navigasyon değişikliklerini dinle
        tauriEvent.listen('webview-navigation', (event) => {
            const { tabId, url } = event.payload || {};
            if (tabId && url && tabId === state.activeTabId) {
                console.log(`Webview navigasyon: ${state.currentUrl} -> ${url}`);
                state.currentUrl = url;
                urlInput.value = url;
                currentUrlSpan.textContent = url;
                
                // Sekme bilgisini güncelle
                const tab = state.tabs.find(t => t.id === tabId);
                if (tab) {
                    tab.url = url;
                    // Yeni URL için sayfa bilgilerini al
                    setTimeout(() => {
                        updateTabInfo(tabId, url);
                    }, 1000);
                }
            }
        });
    }
}

// Slash commands
// Popular Sites
async function loadPopularSites() {
    console.log('🔍 LOADING POPULAR SITES - START');
    console.log('tauriInvoke:', !!tauriInvoke);
    console.log('popularSitesContainer:', !!popularSitesContainer);
    
    if (!tauriInvoke || !popularSitesContainer) {
        console.error('❌ Tauri invoke veya popular sites container eksik');
        return;
    }
    try {
        console.log('📡 Tauri\'dan popüler siteler isteniyor...');
        const sites = await tauriInvoke('get_popular_sites');
        console.log('✅ Popüler siteler alındı:', sites);
        renderPopularSites(sites || []);
    } catch (e) {
        console.error('❌ Popüler siteler alınamadı:', e);
    }
}

function renderPopularSites(sites) {
    console.log('🎨 RENDERING POPULAR SITES');
    console.log('sites:', sites);
    console.log('popularSitesContainer:', !!popularSitesContainer);
    
    if (!popularSitesContainer) {
        console.error('❌ popularSitesContainer bulunamadı');
        return;
    }
    
    popularSitesContainer.innerHTML = '';
    
    if (!sites || sites.length === 0) {
        console.warn('⚠️ Hiç popüler site bulunamadı, test siteleri ekleniyor...');
        // Test için örnek siteler ekle
        sites = [
            { id: 1, title: 'Google', url: 'https://google.com', color: '#4285f4', icon: 'fab fa-google' },
            { id: 2, title: 'YouTube', url: 'https://youtube.com', color: '#ff0000', icon: 'fab fa-youtube' },
            { id: 3, title: 'GitHub', url: 'https://github.com', color: '#333', icon: 'fab fa-github' }
        ];
    }
    
    (sites || []).forEach(site => {
        const el = document.createElement('div');
        el.className = 'site-tile';
        el.dataset.id = site.id;
        el.draggable = true;
        
        console.log(`🏗️ Site oluşturuluyor: ${site.title} (ID: ${site.id})`);
        
        // Click handler (ignore when dragging)
        let clickTimeout = null;
        let isDragStarted = false;
        
        el.addEventListener('mousedown', () => {
            isDragStarted = false;
            clickTimeout = setTimeout(() => {
                isDragStarted = true;
            }, 150); // 150ms sonra drag kabul et
        });
        
        el.addEventListener('mouseup', () => {
            if (clickTimeout) clearTimeout(clickTimeout);
        });
        
        el.addEventListener('click', (e) => {
            console.log(`🖱️ Site tıklandı: ${site.title}`);
            if (isDragStarted || el.classList.contains('dragging')) {
                e.preventDefault();
                return;
            }
            loadUrl(site.url);
        });
        
        el.addEventListener('dragstart', (e) => {
            console.log(`🚀 DRAG START: ${site.title}`);
            onGridDragStart(e);
        });
        
        const color = site.color || '#4b5563';
        const icon = site.icon || 'fas fa-globe';
        el.innerHTML = `
            <div class="site-icon" style="background: ${color};">
                <i class="${icon}"></i>
            </div>
            <span>${site.title}</span>
        `;
        popularSitesContainer.appendChild(el);
    });
    
    // Debug: SortableJS varlığını kontrol et
    console.log('🔍 SortableJS Debug:');
    console.log('window.Sortable var mı?', !!window.Sortable);
    console.log('Sortable version:', window.Sortable?.version);
    
    // SortableJS ile Tauri-uyumlu drag & drop
    if (window.Sortable && popularSitesContainer) {
        console.log('🎯 SortableJS ile drag & drop aktifleştiriliyor...');
        console.log('Container element:', popularSitesContainer);
        
        const sortable = Sortable.create(popularSitesContainer, {
            animation: 150,
            ghostClass: 'sortable-ghost',
            chosenClass: 'sortable-chosen',
            dragClass: 'sortable-drag',
            handle: '.site-tile',
            forceFallback: true, // Tauri için native HTML5 drag yerine fallback kullan
            fallbackTolerance: 0, // Duyarlılığı arttır
            onChoose: function(evt) {
                console.log('🎯 SORTABLE: Item seçildi', evt.item);
            },
            onUnchoose: function(evt) {
                console.log('🔄 SORTABLE: Item seçimi kaldırıldı', evt.item);
            },
            onStart: function(evt) {
                console.log('🚀 SORTABLE: Drag başladı');
                console.log('Item:', evt.item);
                console.log('oldIndex:', evt.oldIndex);
                evt.item.classList.add('dragging');
                popularSitesContainer.classList.add('drag-over');
            },
            onMove: function(evt) {
                console.log('🔄 SORTABLE: Moving...', evt.dragged, evt.related);
                return true; // Hareketin devam etmesine izin ver
            },
            onEnd: async function(evt) {
                console.log('🏁 SORTABLE: Drag bitti');
                console.log('oldIndex:', evt.oldIndex, 'newIndex:', evt.newIndex);
                evt.item.classList.remove('dragging');
                popularSitesContainer.classList.remove('drag-over');
                
                // Yeni sırayı kaydet
                if (evt.newIndex !== evt.oldIndex) {
                    console.log(`📋 Sıra değişti: ${evt.oldIndex} -> ${evt.newIndex}`);
                    const ids = Array.from(popularSitesContainer.children)
                        .map(c => Number(c.dataset.id))
                        .filter(Boolean);
                    
                    console.log('🔄 Yeni sıra:', ids);
                    
                    try {
                        await tauriInvoke('reorder_popular_sites', { ids });
                        console.log('✅ Yeni sıra kaydedildi');
                    } catch (err) {
                        console.error('❌ Sıra kaydedilemedi:', err);
                    }
                }
            }
        });
        
        console.log('✅ SortableJS aktifleştirildi:', sortable);
    } else if (!window.Sortable) {
        console.error('❌ SortableJS bulunamadı! Manuel Drag & Drop aktifleştiriliyor...');
        
        // Manuel Drag & Drop - Tauri için daha uyumlu
        setupManualDragAndDrop();
        
    } else {
        console.warn('⚠️ Container bulunamadı');
    }
    
    console.log(`✅ ${sites.length} popüler site render edildi`);
}

// Manuel Drag & Drop - Tauri için optimize edilmiş (Basitleştirilmiş)
function setupManualDragAndDrop() {
    console.log('🎯 Manuel Drag & Drop aktifleştiriliyor (Basitleştirilmiş)...');
    
    let isDragging = false;
    let draggedElement = null;
    let mouseOffset = { x: 0, y: 0 };
    let startPosition = null;
    
    // Her site kartına long press/drag başlatıcı ekle
    popularSitesContainer.querySelectorAll('.site-tile').forEach((tile, index) => {
        let pressTimer = null;
        let startPos = null;
        
        tile.addEventListener('mousedown', function(e) {
            console.log('🖱️ MANUAL: Mouse down on tile');
            startPos = { x: e.clientX, y: e.clientY };
            
            // Long press detection (300ms)
            pressTimer = setTimeout(() => {
                console.log('⏰ MANUAL: Long press detected - başlatılan drag');
                startDrag(this, e, startPos);
            }, 200);
            
            // Prevent default to avoid conflicts
            e.preventDefault();
        });
        
        tile.addEventListener('mouseup', function(e) {
            if (pressTimer) {
                clearTimeout(pressTimer);
                pressTimer = null;
            }
            
            // Eğer drag yoksa ve kısa basmaydıysa = click
            if (!isDragging) {
                console.log('🖱️ MANUAL: Quick click - site açılacak');
                // Orijinal click event'ini trigger et
                setTimeout(() => tile.click(), 10);
            }
        });
        
        tile.addEventListener('mouseleave', function() {
            if (pressTimer) {
                clearTimeout(pressTimer);
                pressTimer = null;
            }
        });
    });
    
    function startDrag(tile, originalE, startPos) {
        console.log('🚀 MANUAL: Starting drag for', tile);
        
        isDragging = true;
        draggedElement = tile;
        
        // Mouse pozisyonunu kaydet  
        const rect = tile.getBoundingClientRect();
        mouseOffset.x = startPos.x - rect.left;
        mouseOffset.y = startPos.y - rect.top;
        startPosition = Array.from(popularSitesContainer.children).indexOf(tile);
        
        // Sürüklenen elementi stil değiştir
        tile.classList.add('dragging');
        tile.style.position = 'fixed';
        tile.style.zIndex = '1000';
        tile.style.cursor = 'grabbing';
        tile.style.transform = 'rotate(3deg)';
        tile.style.left = (startPos.x - mouseOffset.x) + 'px';
        tile.style.top = (startPos.y - mouseOffset.y) + 'px';
        
        // Container'a visual feedback
        popularSitesContainer.classList.add('drag-over');
        
        // Body-level mouse events
        document.addEventListener('mousemove', handleMouseMove);
        document.addEventListener('mouseup', handleMouseUp);
        
        console.log('🚀 MANUAL: Drag başladı');
    }
    
    function handleMouseMove(e) {
        if (!isDragging || !draggedElement) return;
        
        // Sürüklenen elementi mouse'u takip ettir
        draggedElement.style.left = (e.clientX - mouseOffset.x) + 'px';
        draggedElement.style.top = (e.clientY - mouseOffset.y) + 'px';
        
        console.log('🔄 MANUAL: Mouse moving...');
    }
    
    async function handleMouseUp(e) {
        console.log('🏁 MANUAL: Mouse up');
        
        if (!isDragging || !draggedElement) return;
        
        // Event listener'ları temizle
        document.removeEventListener('mousemove', handleMouseMove);
        document.removeEventListener('mouseup', handleMouseUp);
        
        // Drop pozisyonunu belirle - mouse altındaki tile'ı bul
        const elementBelow = document.elementFromPoint(e.clientX, e.clientY);
        const targetTile = elementBelow?.closest('.site-tile:not(.dragging)');
        let newIndex = startPosition; // Default: aynı pozisyon
        
        if (targetTile) {
            console.log('🎯 MANUAL: Target tile bulundu');
            const allTiles = Array.from(popularSitesContainer.children);
            const targetIndex = allTiles.indexOf(targetTile);
            
            // Mouse position'a göre before/after belirle
            const rect = targetTile.getBoundingClientRect();
            const isAfter = e.clientY > rect.top + rect.height / 2;
            newIndex = isAfter ? targetIndex + 1 : targetIndex;
            
            // Eğer aynı element ya da hemen yanıysa değişiklik yok
            if (newIndex === startPosition || newIndex === startPosition + 1) {
                newIndex = startPosition;
            }
        }
        
        // Elementi eski haline döndür
        draggedElement.style.position = 'static';
        draggedElement.style.zIndex = 'auto';
        draggedElement.style.cursor = 'grab';
        draggedElement.style.transform = 'none';
        draggedElement.style.left = 'auto';
        draggedElement.style.top = 'auto';
        draggedElement.classList.remove('dragging');
        popularSitesContainer.classList.remove('drag-over');
        
        // Pozisyon değiştiyse yeni pozisyona taşı
        if (newIndex !== startPosition) {
            console.log(`📋 MANUAL: Pozisyon değişti: ${startPosition} -> ${newIndex}`);
            
            // DOM'da yeni pozisyona yerleştir
            const allTiles = Array.from(popularSitesContainer.children);
            if (newIndex >= allTiles.length) {
                popularSitesContainer.appendChild(draggedElement);
            } else {
                popularSitesContainer.insertBefore(draggedElement, allTiles[newIndex]);
            }
            
            // Backend'e kaydet
            const ids = Array.from(popularSitesContainer.children)
                .map(c => Number(c.dataset.id))
                .filter(Boolean);
            
            console.log('🔄 MANUAL: Yeni sıra:', ids);
            
            try {
                await tauriInvoke('reorder_popular_sites', { ids });
                console.log('✅ MANUAL: Yeni sıra kaydedildi');
            } catch (err) {
                console.error('❌ MANUAL: Sıra kaydedilemedi:', err);
            }
        }
        
        // Durumu sıfırla
        isDragging = false;
        draggedElement = null;
        startPosition = null;
        
        console.log('✅ MANUAL: Drag tamamlandı');
    }
    
    console.log('✅ Manuel Drag & Drop aktifleştirildi');
}

let draggedGridId = null;
function onGridDragStart(e) {
    const tile = e.currentTarget.closest('.site-tile');
    draggedGridId = tile?.dataset?.id || null;
    if (e.dataTransfer) {
        e.dataTransfer.effectAllowed = 'move';
        try { e.dataTransfer.setData('text/plain', draggedGridId || ''); } catch (_) {}
    }
    if (tile) tile.classList.add('dragging');
}
function onGridDragEnd() {
    const dragging = popularSitesContainer.querySelector('.site-tile.dragging');
    if (dragging) dragging.classList.remove('dragging');
    
    // Remove visual feedback
    if (popularSitesContainer) {
        popularSitesContainer.classList.remove('drag-over');
    }
    
    draggedGridId = null;
}
function getGridAfterElement(container, x, y) {
    const els = [...container.querySelectorAll('.site-tile:not(.dragging)')];
    let closest = { distance: Number.POSITIVE_INFINITY, element: null };
    for (const child of els) {
        const box = child.getBoundingClientRect();
        const cx = box.left + box.width / 2;
        const cy = box.top + box.height / 2;
        const dx = x - cx;
        const dy = y - cy;
        const dist = Math.hypot(dx, dy);
        if (dist < closest.distance) {
            closest = { distance: dist, element: child };
        }
    }
    return closest.element;
}
function onGridContainerDragOver(e) {
    e.preventDefault();
    if (e.dataTransfer) e.dataTransfer.dropEffect = 'move';
    
    // Add visual feedback
    if (!popularSitesContainer.classList.contains('drag-over')) {
        popularSitesContainer.classList.add('drag-over');
    }
    
    const dragging = popularSitesContainer.querySelector('.site-tile.dragging');
    if (!dragging) return;
    
    const afterElement = getGridAfterElement(popularSitesContainer, e.clientX, e.clientY);
    if (!afterElement || afterElement === dragging) return;
    
    const box = afterElement.getBoundingClientRect();
    const before = (e.clientY < box.top + box.height / 2) || (e.clientX < box.left + box.width / 2);
    if (before) popularSitesContainer.insertBefore(dragging, afterElement);
    else popularSitesContainer.insertBefore(dragging, afterElement.nextSibling);
}
async function onGridContainerDrop(e) {
    e.preventDefault();
    
    // Remove visual feedback immediately
    if (popularSitesContainer) {
        popularSitesContainer.classList.remove('drag-over');
    }
    
    const ids = Array.from(popularSitesContainer.children).map(c => Number(c.dataset.id)).filter(Boolean);
    try {
        await tauriInvoke('reorder_popular_sites', { ids });
    } catch (err) { console.error('Reorder kaydedilemedi:', err); }
    onGridDragEnd();
}

async function openPopularModal() {
    await refreshPopularModalList();
    if (popularModal) popularModal.classList.remove('hidden');
}

function closePopularModal() { if (popularModal) popularModal.classList.add('hidden'); }

function resetPopularForm() {
    if (!popularIdInput) return;
    popularIdInput.value = '';
    popularTitleInput.value = '';
    popularUrlInput.value = '';
    popularColorInput.value = '';
    popularIconInput.value = '';
}

async function refreshPopularModalList() {
    if (!tauriInvoke || !popularList) return;
    try {
        const sites = await tauriInvoke('get_popular_sites');
        popularList.innerHTML = '';
        (sites || []).forEach(site => {
            const row = document.createElement('div');
            row.className = 'popular-row';
            row.draggable = true;
            row.dataset.id = site.id;
            row.style.display = 'flex';
            row.style.alignItems = 'center';
            row.style.justifyContent = 'space-between';
            row.style.padding = '8px 0';
            row.style.gap = '8px';
            row.innerHTML = `
                <div style="display:flex;align-items:center;gap:10px;flex:1;min-width:0;">
                    <i class="fas fa-grip-vertical" style="cursor:grab;color:var(--text-muted);"></i>
                    <div class="site-icon" style="width:24px;height:24px;background:${site.color || '#4b5563'};">
                        <i class="${site.icon || 'fas fa-globe'}" style="font-size:12px;"></i>
                    </div>
                    <div style="min-width:0;">
                        <div style="font-weight:600; white-space:nowrap; overflow:hidden; text-overflow:ellipsis;">${site.title}</div>
                        <div style="font-size:12px;color:var(--text-secondary); white-space:nowrap; overflow:hidden; text-overflow:ellipsis;">${site.url}</div>
                    </div>
                </div>
                <div style="display:flex;gap:6px;">
                    <button class="control-btn" title="Düzenle" data-action="edit"><i class="fas fa-pen"></i></button>
                    <button class="control-btn" title="Sil" data-action="del"><i class="fas fa-trash"></i></button>
                </div>
            `;
            // Edit
            row.querySelector('[data-action="edit"]').addEventListener('click', () => fillPopularForm(site));
            // Delete
            row.querySelector('[data-action="del"]').addEventListener('click', async () => {
                try { await tauriInvoke('delete_popular_site', { id: site.id }); } catch (e) { console.error(e); }
                await refreshPopularModalList();
                await loadPopularSites();
            });

            // Drag start
            row.addEventListener('dragstart', onPopularDragStart);

            popularList.appendChild(row);
        });
        // Container-level DnD
        popularList.addEventListener('dragover', onPopularContainerDragOver);
        popularList.addEventListener('drop', onPopularContainerDrop);
        popularList.addEventListener('dragend', onPopularDragEnd);
    } catch (e) {
        console.error('Popüler siteler listelenemedi:', e);
    }
}

let draggedPopularId = null;
function onPopularDragStart(e) {
    const row = e.currentTarget.closest('.popular-row');
    draggedPopularId = row?.dataset?.id || null;
    if (e.dataTransfer) {
        e.dataTransfer.effectAllowed = 'move';
        try { e.dataTransfer.setData('text/plain', draggedPopularId || ''); } catch (_) {}
    }
    if (row) row.classList.add('dragging');
}
function onPopularDragEnd() {
    const dragging = popularList.querySelector('.popular-row.dragging');
    if (dragging) dragging.classList.remove('dragging');
    draggedPopularId = null;
}
function getDragAfterElement(container, y) {
    const els = [...container.querySelectorAll('.popular-row:not(.dragging)')];
    let closest = { offset: Number.NEGATIVE_INFINITY, element: null };
    for (const child of els) {
        const box = child.getBoundingClientRect();
        const offset = y - (box.top + box.height / 2);
        if (offset < 0 && offset > closest.offset) {
            closest = { offset, element: child };
        }
    }
    return closest.element;
}
function onPopularContainerDragOver(e) {
    e.preventDefault();
    if (e.dataTransfer) e.dataTransfer.dropEffect = 'move';
    const dragging = popularList.querySelector('.popular-row.dragging');
    if (!dragging) return;
    const afterElement = getDragAfterElement(popularList, e.clientY);
    if (afterElement == null) {
        popularList.appendChild(dragging);
    } else {
        popularList.insertBefore(dragging, afterElement);
    }
}
async function onPopularContainerDrop(e) {
    e.preventDefault();
    // Persist new order
    const ids = Array.from(popularList.children).map(c => Number(c.dataset.id)).filter(Boolean);
    try {
        await tauriInvoke('reorder_popular_sites', { ids });
        await loadPopularSites();
    } catch (err) { console.error('Reorder kaydedilemedi:', err); }
    onPopularDragEnd();
}

function fillPopularForm(site) {
    popularIdInput.value = site.id;
    popularTitleInput.value = site.title || '';
    popularUrlInput.value = site.url || '';
    popularColorInput.value = site.color || '';
    popularIconInput.value = site.icon || '';
}

async function savePopularSite() {
    const id = popularIdInput.value ? Number(popularIdInput.value) : null;
    const title = (popularTitleInput.value || '').trim();
    const url = (popularUrlInput.value || '').trim();
    const color = (popularColorInput.value || '').trim();
    const icon = (popularIconInput.value || '').trim();
    if (!title || !url) { alert('Başlık ve URL zorunlu'); return; }
    try {
        await tauriInvoke('save_popular_site', { id, title, url, color: color || null, icon: icon || null, sort_order: null });
        resetPopularForm();
        await refreshPopularModalList();
        await loadPopularSites();
    } catch (e) {
        console.error('Popüler site kaydedilemedi:', e);
    }
}

function schedulePopularAutoSave() {
    if (!popularTitleInput || !popularUrlInput) return;
    clearTimeout(popularAutoSaveTimer);
    popularAutoSaveTimer = setTimeout(async () => {
        const t = (popularTitleInput.value || '').trim();
        const u = (popularUrlInput.value || '').trim();
        // Kaydetme koşulu: başlık ve URL dolu olmalı
        if (!t || !u) return;
        const id = popularIdInput.value ? Number(popularIdInput.value) : null;
        const color = (popularColorInput.value || '').trim();
        const icon = (popularIconInput.value || '').trim();
        try {
            const newId = await tauriInvoke('save_popular_site', { id, title: t, url: u, color: color || null, icon: icon || null, sort_order: null });
            if (!id && newId) popularIdInput.value = Number(newId);
            await refreshPopularModalList();
            await loadPopularSites();
        } catch (e) {
            console.error('Otomatik kaydetme hatası:', e);
        }
    }, 400);
}
const SLASH_COMMANDS = [
    { cmd: '/ozetle', hint: 'Metni kısa özetle' },
    { cmd: '/acikla', hint: 'Detaylı açıkla' },
    { cmd: '/madde', hint: 'Maddeler halinde yaz' },
    { cmd: '/kaynakekle', hint: 'Kaynakları belirt' },
    { cmd: '/kisalt', hint: 'Daha kısa yaz' },
    { cmd: '/uzat', hint: 'Daha detaylı yaz' },
];

function handleSlashInput() {
    const val = messageInput.value;
    const trimmed = val.trimStart();
    if (!trimmed.startsWith('/')) { hideSlashSuggestions(); return; }
    const firstToken = trimmed.split(/\s|\n/)[0];
    const query = firstToken.slice(1).toLowerCase();
    const list = SLASH_COMMANDS.filter(x => x.cmd.slice(1).toLowerCase().startsWith(query));
    renderSlashSuggestions(list);
}

function renderSlashSuggestions(items) {
    if (!slashBox) return;
    if (!items || items.length === 0) { hideSlashSuggestions(); return; }
    slashBox.innerHTML = '';
    items.forEach((it, idx) => {
        const div = document.createElement('div');
        div.className = 'item' + (idx === 0 ? ' active' : '');
        div.dataset.index = String(idx);
        div.dataset.cmd = it.cmd;
        div.innerHTML = `<span class="cmd">${it.cmd}</span><span class="hint">${it.hint}</span>`;
        div.addEventListener('mousedown', (e) => {
            e.preventDefault(); // keep focus
            insertSlashCommand(it.cmd);
        });
        slashBox.appendChild(div);
    });
    slashBox.classList.remove('hidden');
    state.slashVisible = true;
    state.slashIndex = 0;
}

function hideSlashSuggestions() {
    if (!slashBox) return;
    slashBox.classList.add('hidden');
    state.slashVisible = false;
    state.slashIndex = -1;
}

function moveSlashSelection(delta) {
    if (!slashBox) return;
    const items = Array.from(slashBox.querySelectorAll('.item'));
    if (items.length === 0) return;
    state.slashIndex = (state.slashIndex + delta + items.length) % items.length;
    items.forEach((el, i) => el.classList.toggle('active', i === state.slashIndex));
}

function getSlashSelectedItem() {
    if (!slashBox) return null;
    const items = Array.from(slashBox.querySelectorAll('.item'));
    if (items.length === 0) return null;
    const idx = Math.max(0, Math.min(state.slashIndex ?? 0, items.length - 1));
    return items[idx];
}

function insertSlashCommand(cmd) {
    const val = messageInput.value;
    const leadingSpaces = val.match(/^\s*/)?.[0] || '';
    const trimmed = val.slice(leadingSpaces.length);
    if (trimmed.startsWith('/')) {
        const rest = trimmed.split(/\s|\n/).slice(1).join(' ');
        messageInput.value = leadingSpaces + cmd + (rest ? (' ' + rest) : ' ');
    } else {
        messageInput.value = leadingSpaces + cmd + ' ' + trimmed;
    }
    messageInput.focus();
    messageInput.dispatchEvent(new Event('input'));
    hideSlashSuggestions();
}

function parseSlashDirective(text) {
    const m = text.match(/^\s*\/(\w+)\b(.*)$/);
    if (!m) return { clean: text, directive: null };
    const keyword = m[1].toLowerCase();
    const rest = (m[2] || '').trim();
    let directive = null;
    switch (keyword) {
        case 'ozetle':
            directive = 'Biçim: Kısa ve öz bir özet ver.';
            break;
        case 'acikla':
            directive = 'Biçim: Detaylı ve açıklayıcı anlat.';
            break;
        case 'madde':
            directive = 'Biçim: Maddeler halinde açıkla.';
            break;
        case 'kaynakekle':
            directive = 'Biçim: Varsa kaynak/bağlantıları ekle.';
            break;
        case 'kisalt':
            directive = 'Biçim: Daha kısa yaz.';
            break;
        case 'uzat':
            directive = 'Biçim: Daha detaylı yaz.';
            break;
        default:
            return { clean: text, directive: null };
    }
    const clean = rest || inferDefaultQuestionFor(keyword);
    return { clean, directive };
}

function inferDefaultQuestionFor(keyword) {
    switch (keyword) {
        case 'ozetle': return 'Bu sayfayı özetler misin?';
        case 'acikla': return 'Bu içeriği detaylı açıklar mısın?';
        case 'madde': return 'Bu içeriği maddeler halinde açıklar mısın?';
        case 'kaynakekle': return 'Bu içerikle ilgili kaynak/bağlantıları ekler misin?';
        case 'kisalt': return 'Bu yanıtı daha kısa yazar mısın?';
        case 'uzat': return 'Bu yanıtı daha detaylı yazar mısın?';
        default: return '';
    }
}

function buildQuestionWithDirective(clean, directive) {
    if (!directive) return clean;
    if (!clean || !clean.trim()) return directive;
    return `${directive}\n\n${clean}`;
}

async function loadOllamaModels() {
    if (!tauriInvoke) {
        console.error('Tauri invoke fonksiyonu bulunamadı');
        return [];
    }

    try {
        console.log('=== OLLAMA MODEL YÜKLEME BAŞLIYOR ===');
        console.log('Tauri invoke çağrılıyor: get_ollama_models');
        // Base URL backend ayarından gelir; komut onu kullanır
        const models = await tauriInvoke('get_ollama_models');

        console.log('Modeller yüklendi:', models);
        state.availableModels = Array.isArray(models) ? models : [];

        // Bu fonksiyon yalnızca veriyi döndürür; UI güncellemesi çağıran fonksiyonda yapılır
        return state.availableModels;
    } catch (error) {
        console.error('Ollama modelleri yüklenirken hata:', error);
        // UI durumunu burada bozmayalım; çağıran taraf uygun mesajı set edecek
        return [];
    }
}

async function loadOpenRouterModels() {
    if (!tauriInvoke) {
        console.error('Tauri invoke fonksiyonu bulunamadı');
        return [];
    }
    try {
        const models = await tauriInvoke('get_openrouter_models');
        return models || [];
    } catch (e) {
        console.error('OpenRouter modelleri yüklenirken hata:', e);
        return [];
    }
}

async function loadAllModelsWithOllama() {
    const token = ++modelsRequestToken;
    if (!modelSelect) {
        console.error('Model select element bulunamadı');
        return;
    }
    
    console.log('Model yükleme başlıyor... Ollama etkin:', state.ollamaEnabled);
    modelSelect.innerHTML = '<option value="">Modeller yükleniyor...</option>';
    
    let ollama = [];
    
    // Sadece LOCAL mod: sadece Ollama modellerini getir
    if (state.ollamaEnabled) {
        try {
            ollama = await loadOllamaModels();
        } catch (e) {
            console.error('Ollama modelleri yüklenemedi:', e);
            ollama = [];
        }
    }
    // Stale kontrolü
    if (token !== modelsRequestToken) { console.warn('Stale model load sonucu yok sayıldı'); return; }
    
    modelSelect.innerHTML = '';
    const addOpt = (value, label) => {
        const o = document.createElement('option');
        o.value = value; o.textContent = label; modelSelect.appendChild(o);
    };

    // Sadece LOCAL mod: yalnızca Ollama modellerini göster
    if (state.ollamaEnabled) {
        // Gelen sonuç boşsa cache'deki son geçerli listeyi kullan
        if ((!ollama || ollama.length === 0) && state.ollamaModelsCache && state.ollamaModelsCache.length > 0) {
            console.warn('Ollama boş döndü, cache kullanılıyor');
            ollama = state.ollamaModelsCache;
        }

        if (ollama && Array.isArray(ollama) && ollama.length > 0) {
            // Cache'i güncelle
            state.ollamaModelsCache = ollama;
            // İsteğe bağlı başlık eklemeyelim; sade liste
            ollama.forEach(m => {
                addOpt(`ollama:${m.name}`, m.name);
            });
            // Varsayılanı ilk Ollama modeli yap
            const first = `ollama:${ollama[0].name}`;
            state.currentModel = first;
            modelSelect.value = first;
        } else {
            addOpt('ollama:none', 'Ollama modeli bulunamadı');
            state.currentModel = 'ollama:none';
            modelSelect.value = 'ollama:none';
        }
        const label = modelSelect.options[modelSelect.selectedIndex]?.textContent || 'Yerel Model';
        updateChatStatus(`Model: ${label}`);
        return;
    }

    // ONLINE modda bu fonksiyon çağrılmamalı
    modelSelect.innerHTML = '';
}

// Settings helpers
async function openSettings() {
    try {
        if (tauriInvoke) {
            const current = await tauriInvoke('get_ollama_base_url');
            if (ollamaUrlInput) ollamaUrlInput.value = current || 'http://localhost:11434';
        }
    } catch (e) {
        console.error('Ayarlar okunamadı:', e);
    }
    
    // Ollama durumunu yükle
    const savedMode = localStorage.getItem('ai_mode');
    state.ollamaEnabled = savedMode === 'local';
    if (enableOllamaCheckbox) enableOllamaCheckbox.checked = state.ollamaEnabled;
    
    // Ollama ayarları görünürlüğü
    updateOllamaSettingsVisibility();
    
    settingsModal.classList.remove('hidden');
}

function toggleOllama() {
    state.ollamaEnabled = enableOllamaCheckbox.checked;
    console.log('Ollama toggle değişti:', state.ollamaEnabled);
    updateOllamaSettingsVisibility();
    updateModelSelector();
}

function setMode(mode) {
    // mode: 'online' | 'local'
    const online = mode === 'online';
    state.ollamaEnabled = !online;
    localStorage.setItem('ai_mode', online ? 'online' : 'local');
    // UI state
    if (modeOnlineBtn && modeLocalBtn) {
        modeOnlineBtn.classList.toggle('active', online);
        modeLocalBtn.classList.toggle('active', !online);
    }
    updateModelSelector();
}

function updateOllamaSettingsVisibility() {
    if (ollamaSettings) {
        ollamaSettings.style.display = state.ollamaEnabled ? 'block' : 'none';
    }
}

function updateModelSelector() {
    console.log('updateModelSelector çağrıldı, ollamaEnabled:', state.ollamaEnabled);
    const modelSelectorEl = document.querySelector('.model-selector');
    if (!modelSelectorEl) { console.error('Model selector element bulunamadı'); return; }

    if (state.ollamaEnabled) {
        // LOCAL mod: yalnızca Ollama modellerini göster
        modelSelectorEl.classList.add('visible');
        loadAllModelsWithOllama();
    } else {
        // ONLINE mod: model seçici tamamen gizli
        modelSelectorEl.classList.remove('visible');
        state.currentModel = 'openrouter:google/gemini-2.0-flash-exp:free';
    }
}

function closeSettings() {
    settingsModal.classList.add('hidden');
}

async function saveSettings() {
    try {
        // Ollama durumunu kaydet
        localStorage.setItem('ollama_enabled', state.ollamaEnabled.toString());
        
        // Ollama URL'i kaydet (eğer etkinse)
        if (state.ollamaEnabled) {
            const value = (ollamaUrlInput?.value || '').trim();
            if (value && tauriInvoke) {
                await tauriInvoke('set_ollama_base_url', { value });
            }
        }
        
        updateChatStatus('Ayarlar kaydedildi');
        closeSettings();
        
        // Model seçiciyi güncelle
        updateModelSelector();
        
    } catch (e) {
        console.error('Ayar kaydedilemedi:', e);
        updateChatStatus('Ayar kaydedilemedi', 'error');
    }
}

function handleUrlSubmit() {
    const input = urlInput.value.trim();
    if (!input) return;

    // URL mi arama mı ayırt et
    const looksLikeUrl = /^(https?:\/\/)/i.test(input) || (input.includes('.') && !input.includes(' '));
    const target = looksLikeUrl 
        ? input 
        : `https://www.google.com/search?q=${encodeURIComponent(input)}`;

    loadUrl(target);
}

function loadUrl(url) {
    // URL'yi düzelt
    if (!url.startsWith('http://') && !url.startsWith('https://')) {
        url = 'https://' + url;
    }

    // Bu çağrı sırasında aktif sekmenin id'sini sabitle
    const targetTabId = state.activeTabId;
    if (!targetTabId) {
        ensureActiveTab();
    }

    state.currentUrl = url;
    urlInput.value = url;

    updateStatus('loading');
    currentUrlSpan.textContent = url;
    
    if (tauriInvoke) {
        tauriInvoke('open_or_navigate_browser_tab', { tabId: targetTabId, url })
            .then(() => {
                hideWelcomeScreen();
                tauriInvoke('show_only_tab', { tabId: targetTabId }).catch(e => console.error("Failed to show tab:", e));
                updateBrowserViewPositionAndSize();
                
                // Update tab information after page loads
                // Try immediate update and then a delayed one for fallback
                updateTabInfo(targetTabId, url);
                setTimeout(() => {
                    updateTabInfo(targetTabId, url);
                }, 3000);
            })
            .catch((err) => {
                console.error('Webview yükleme hatası:', err);
                updateStatus('error');
                addChatMessage('system', `URL yüklenirken hata oluştu: ${err}`);
                goToHome();
            });
    } else {
        console.warn("Tauri API not available for loading URL.");
    }
}

async function handleChatSubmit() {
    const message = messageInput.value.trim();
    if (!message) return;
    
    if (!state.currentUrl) {
        addChatMessage('system', 'Lütfen önce bir web sayfası açın!');
        return;
    }
    
    if (!state.currentModel) {
        addChatMessage('system', 'Lütfen bir AI model seçin!');
        return;
    }
    
    if (!tauriInvoke) {
        addChatMessage('system', 'Tauri API kullanılamıyor!');
        return;
    }
    
    // Slash komutunu işle ve modele per-turn talimat ekle
    const { clean, directive } = parseSlashDirective(message);

    // Add user message to chat (kullanıcı girdisini olduğu gibi gösteriyoruz)
    addChatMessage('user', message);
    messageInput.value = '';
    messageInput.style.height = 'auto';
    
    // Disable send button
    sendButton.disabled = true;
    setChatLoading(true);
    
    try {
        console.group('CHAT REQUEST');
        console.log('mode:', state.ollamaEnabled ? 'local' : 'online');
        console.log('url:', state.currentUrl);
        console.log('model:', state.currentModel);
        console.groupEnd();
        // Model seçimine göre uygun komutu çağır
        if (state.ollamaEnabled && state.currentModel && state.currentModel.startsWith('ollama:')) {
            // Ollama kullan
            await tauriInvoke('ask_question', {
                url: state.currentUrl,
                question: buildQuestionWithDirective(clean, directive),
                model: state.currentModel.replace('ollama:', '')
            });
        } else {
            // OpenRouter kullan (varsayılan)
            const model = state.currentModel ? state.currentModel.replace('openrouter:', '') : 'google/gemini-2.0-flash-exp:free';
            await tauriInvoke('ask_question_openrouter', {
                url: state.currentUrl,
                question: buildQuestionWithDirective(clean, directive),
                model: model
            });
        }
        // Streaming yanıtları event listener'lar yönetiyor
        
    } catch (error) {
        console.error('Soru sorulurken hata:', error);
        addChatMessage('system', `Hata: ${error.message || error}`);
        updateChatStatus('Hata oluştu', 'error');
        setChatLoading(false);
        sendButton.disabled = false;
    }
}

function addChatMessage(type, content) {
    const messageDiv = document.createElement('div');
    messageDiv.className = `message ${type}`;
    
    const timestamp = new Date().toLocaleTimeString('tr-TR', { 
        hour: '2-digit', 
        minute: '2-digit' 
    });
    
    if (type === 'system') {
        messageDiv.innerHTML = `
            <div class="message-content">${content}</div>
        `;
    } else {
        const senderName = type === 'user' ? 'Sen' : 'Bot';
        const iconClass = type === 'user' ? 'fa-user' : 'fa-robot';
        messageDiv.innerHTML = `
            <div class="message-header">
                <i class="fas ${iconClass}"></i>
                <span>${senderName}</span>
                <span>${timestamp}</span>
            </div>
            <div class="message-content">${content}</div>
        `;
    }
    
    chatMessages.appendChild(messageDiv);
    
    // Scroll to bottom
    chatMessages.scrollTop = chatMessages.scrollHeight;

    // Return the message content div for streaming
    return messageDiv.querySelector('.message-content');
}

function clearChat() {
    chatMessages.innerHTML = `
        <div class="system-message">
            <i class="fas fa-info-circle"></i>
            <span>Sohbet temizlendi. Açtığınız web sayfası hakkında soru sorabilirsiniz.</span>
        </div>
    `;
}

function toggleChat() {
    state.chatVisible = !state.chatVisible;

    if (state.chatVisible) {
        chatPanel.classList.remove('hidden');
        chatToggle.classList.add('active');
    } else {
        chatPanel.classList.add('hidden');
        chatToggle.classList.remove('active');
    }
    // DO NOT call updateBrowserViewPositionAndSize() here directly.
    // The 'transitionend' event listener will handle it.
}

function updateStatus(type) {
    statusIndicator.className = `status-${type}`;
    statusIndicator.textContent = ''; // Artık metin göstermiyoruz
}

function updateChatStatus(message, type = '') {
    const statusText = document.querySelector('.status-text');
    if (statusText) {
        statusText.textContent = message;
        statusText.className = `status-text ${type}`;
    }
}

function setChatLoading(isLoading, label = '') {
    const spinner = document.getElementById('chat-status-spinner');
    const statusText = document.querySelector('.status-text');
    if (spinner) spinner.style.display = isLoading ? 'inline-block' : 'none';
    if (statusText) {
        statusText.textContent = isLoading ? label : 'Hazır';
        statusText.className = `status-text ${isLoading ? 'processing' : ''}`;
    }
}

function showWelcomeScreen() {
    welcomeScreen.style.display = 'flex';
    if (browserViewContainer) browserViewContainer.style.display = 'none';
    if (tauriInvoke) tauriInvoke('hide_webview').catch(e => console.error(e));
    currentUrlSpan.textContent = '';
    updateStatus('ready');
}

function hideWelcomeScreen() {
    welcomeScreen.style.display = 'none';
    if (browserViewContainer) browserViewContainer.style.display = 'block';
}


// Helper to resize and position the webview
function updateBrowserViewPositionAndSize() {
    if (!tauriInvoke || !state.currentUrl || !state.activeTabId) return;

    const contentEl = document.querySelector('.browser-content');
    if (contentEl) {
        const rect = contentEl.getBoundingClientRect();
        const params = {
            tabId: state.activeTabId,
            x: Math.round(rect.left),
            y: Math.round(rect.top),
            width: Math.round(rect.width),
            height: Math.round(rect.height)
        };
        tauriInvoke('reposition_webview', params).catch(err => {
            console.error("Failed to reposition webview:", err);
        });
    }
}

function goToHome() {
    showWelcomeScreen();
    state.currentUrl = '';
    urlInput.value = '';
}

// Quick links function (called from HTML)
window.loadUrl = loadUrl;

// Keyboard shortcuts
document.addEventListener('keydown', (e) => {
    // Ctrl/Cmd + R: Refresh
    if ((e.ctrlKey || e.metaKey) && e.key === 'r') {
        e.preventDefault();
        if (state.currentUrl) {
            loadUrl(state.currentUrl);
        }
    }
    
    // Ctrl/Cmd + L: Focus URL bar
    if ((e.ctrlKey || e.metaKey) && e.key === 'l') {
        e.preventDefault();
        urlInput.focus();
        urlInput.select();
    }
    
    // Cmd + Alt (Option) + I: Devtools Toggle
    if (e.metaKey && e.altKey && e.key.toLowerCase() === 'i') {
        e.preventDefault();
        try {
            if (window.__TAURI__?.webview?.internalToggleDevtools) {
                window.__TAURI__.webview.internalToggleDevtools();
            }
        } catch (err) {
            console.error('Devtools toggle hatası:', err);
        }
    }

    // Esc: Close chat or clear input
    if (e.key === 'Escape') {
        if (messageInput && messageInput.value) {
            messageInput.value = '';
            messageInput.style.height = 'auto';
        } else if (state.chatVisible && window.innerWidth <= 768) {
            toggleChat();
        }
    }
});

// Tabs helpers
function createNewTab() {
    const id = 'tab-' + Math.random().toString(36).slice(2, 8);
    state.tabs.push({ id, title: 'Yeni Sekme', url: '', favicon: '' });
    state.activeTabId = id;
    // Yeni sekmede her zaman anasayfa görünsün
    goToHome();
    // Mevcut tüm webview'leri gizle; yeni sekme daha sonra URL açınca görünür olacak
    if (tauriInvoke) {
        tauriInvoke('show_only_tab', { tabId: id }).catch(()=>{});
    }
    renderTabs();
}

// Update tab information when page loads
async function updateTabInfo(tabId, url) {
    if (!tauriInvoke) return;
    
    try {
        // Get page title and favicon from Tauri backend
        const pageInfo = await tauriInvoke('get_page_info', { tabId, url });
        
        // Find and update the tab
        const tab = state.tabs.find(t => t.id === tabId);
        if (tab) {
            tab.title = pageInfo.title || new URL(url).hostname || 'Yeni Sekme';
            tab.favicon = pageInfo.favicon || '';
            tab.url = url;
            
            // Re-render tabs to show updated info
            renderTabs();
        }
    } catch (error) {
        console.log('Sayfa bilgisi alınamadı:', error);
        // Fallback: just use hostname as title
        const tab = state.tabs.find(t => t.id === tabId);
        if (tab) {
            try {
                tab.title = new URL(url).hostname;
                tab.url = url;
                renderTabs();
            } catch (e) {
                // If URL parsing fails, keep original title
            }
        }
    }
}

function switchTab(id) {
    state.activeTabId = id;
    const tab = state.tabs.find(t => t.id === id);
    state.currentUrl = tab?.url || '';
    urlInput.value = state.currentUrl;
    currentUrlSpan.textContent = state.currentUrl;
    // URL yoksa welcome göster, varsa webview göster
    if (!state.currentUrl) {
        showWelcomeScreen();
    } else {
        hideWelcomeScreen();
    }
    renderTabs();
    if (tauriInvoke && id) {
        tauriInvoke('show_only_tab', { tabId: id }).catch(()=>{});
        // Sekme değişiminde pozisyonu güncelle
        updateBrowserViewPositionAndSize();
    }
}

function closeTab(id) {
    const idx = state.tabs.findIndex(t => t.id === id);
    if (idx === -1) return;
    const wasActive = state.activeTabId === id;
    const closingTab = state.tabs[idx];
    // Backend cache temizliği (URL mevcutsa)
    if (tauriInvoke && closingTab && closingTab.url) {
        tauriInvoke('clear_cache_for_url', { url: closingTab.url }).catch(err => console.warn('Cache temizleme hatası:', err));
    }
    state.tabs.splice(idx, 1);
    if (wasActive) {
        const fallback = state.tabs[idx] || state.tabs[idx - 1];
        state.activeTabId = fallback ? fallback.id : '';
        if (state.activeTabId) switchTab(state.activeTabId);
    }
    renderTabs();
}

function ensureActiveTab() {
    if (!state.activeTabId) createNewTab();
}

function renderTabs() {
    if (!tabsContainer) return;
    tabsContainer.innerHTML = '';
    state.tabs.forEach(tab => {
        const el = document.createElement('div');
        el.className = 'tab-item' + (tab.id === state.activeTabId ? ' active' : '');
        
        // Create favicon element
        let faviconHtml = '';
        if (tab.favicon && tab.favicon !== '') {
            faviconHtml = `<img src="${tab.favicon}" class="tab-favicon" alt="favicon" onerror="this.style.display='none'">`;
        } else {
            // Default icon for new tabs or when favicon is not available
            faviconHtml = `<i class="fas fa-globe tab-default-icon"></i>`;
        }
        
        el.innerHTML = `
            ${faviconHtml}
            <span class="tab-title">${tab.title}</span> 
            <button class="tab-close" title="Kapat">×</button>
        `;
        
        el.addEventListener('click', (e) => {
            if (e.target && e.target.classList.contains('tab-close')) {
                e.stopPropagation();
                closeTab(tab.id);
            } else {
                switchTab(tab.id);
            }
        });
        tabsContainer.appendChild(el);
    });
}

// Log Tauri environment on load
console.log('=== TAURI ENVIRONMENT CHECK ===');
console.log('window.__TAURI__:', window.__TAURI__);
console.log('window.__TAURI_INTERNALS__:', window.__TAURI_INTERNALS__);
console.log('================================');
