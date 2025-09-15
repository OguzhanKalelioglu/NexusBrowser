// Global state
const state = {
    currentUrl: '',
    currentModel: '',
    availableModels: [],
    isLoading: false,
    chatVisible: true,
    tabs: [], // {id, title, url}
    activeTabId: ''
};

// DOM Elements
let urlInput, browserFrame, welcomeScreen, chatMessages, messageInput, 
    sendButton, statusIndicator, currentUrlSpan, modelSelect, chatPanel,
    chatToggle, loadingOverlay;
let tabsContainer, newTabBtn;

// Tauri API bekle
let tauriReady = false;
let tauriInvoke = null;

// Wait for Tauri to be ready
function waitForTauri() {
    return new Promise((resolve) => {
        // Check if Tauri is already available
        if (window.__TAURI_INTERNALS__) {
            console.log('Tauri INTERNALS API bulundu');
            tauriInvoke = window.__TAURI_INTERNALS__.invoke;
            tauriReady = true;
            resolve();
        } else if (window.__TAURI__) {
            console.log('Tauri API bulundu');
            tauriInvoke = window.__TAURI__.core?.invoke || window.__TAURI__.invoke;
            tauriReady = true;
            resolve();
        } else {
            // Wait for Tauri to load
            let attempts = 0;
            const checkInterval = setInterval(() => {
                attempts++;
                console.log(`Tauri API bekleniyor... (deneme ${attempts})`);
                
                if (window.__TAURI_INTERNALS__) {
                    console.log('Tauri INTERNALS API yüklendi!');
                    tauriInvoke = window.__TAURI_INTERNALS__.invoke;
                    tauriReady = true;
                    clearInterval(checkInterval);
                    resolve();
                } else if (window.__TAURI__) {
                    console.log('Tauri API yüklendi!');
                    tauriInvoke = window.__TAURI__.core?.invoke || window.__TAURI__.invoke;
                    tauriReady = true;
                    clearInterval(checkInterval);
                    resolve();
                } else if (attempts > 20) {
                    console.error('Tauri API 10 saniye içinde yüklenemedi');
                    clearInterval(checkInterval);
                    resolve(); // Continue without Tauri
                }
            }, 500);
        }
    });
}

// Initialize app
document.addEventListener('DOMContentLoaded', async () => {
    console.log('DOM yüklendi, app başlatılıyor...');
    
    initializeElements();
    setupEventListeners();
    
    // Show welcome screen initially
    showWelcomeScreen();
    
    // Wait for Tauri API
    await waitForTauri();

    // Devtools otomatik açma kaldırıldı (Option+Cmd+I ile aç/kapat)
    
    if (tauriReady && tauriInvoke) {
        console.log('Tauri API hazır, modeller yükleniyor...');
        await loadOllamaModels();
        // İlk sekme
        createNewTab();
        // Listen menu events to open settings
        try {
            if (window.__TAURI__?.event?.listen) {
                await window.__TAURI__.event.listen('open-settings', () => {
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
    browserFrame = document.getElementById('browser-frame');
    welcomeScreen = document.getElementById('welcome-screen');
    chatMessages = document.getElementById('chat-messages');
    messageInput = document.getElementById('message-input');
    sendButton = document.getElementById('send-btn');
    statusIndicator = document.getElementById('status-indicator');
    currentUrlSpan = document.getElementById('current-url');
    modelSelect = document.getElementById('model-select');
    chatPanel = document.getElementById('chat-panel');
    chatToggle = document.getElementById('chat-toggle');
    loadingOverlay = document.getElementById('loading-overlay');
    // Settings
    settingsBtn = document.getElementById('settings-btn');
    settingsModal = document.getElementById('settings-modal');
    settingsClose = document.getElementById('settings-close');
    settingsSave = document.getElementById('settings-save');
    ollamaUrlInput = document.getElementById('ollama-url');
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
        if (browserFrame.contentWindow) {
            browserFrame.contentWindow.history.back();
        }
    });
    
    document.getElementById('forward-btn').addEventListener('click', () => {
        if (browserFrame.contentWindow) {
            browserFrame.contentWindow.history.forward();
        }
    });
    
    document.getElementById('refresh-btn').addEventListener('click', () => {
        if (state.currentUrl) {
            loadUrl(state.currentUrl);
        }
    });
    
    // Open externally (iframe engeli olan siteler için)
    const openExternalBtn = document.getElementById('open-external');
    if (openExternalBtn) {
        openExternalBtn.addEventListener('click', async () => {
            if (!state.currentUrl) return;
            try {
                if (tauriInvoke) {
                    await tauriInvoke('open_in_browser', { url: state.currentUrl });
                } else {
                    window.open(state.currentUrl, '_blank');
                }
            } catch (err) {
                console.error('Harici tarayıcıda açma hatası:', err);
                addChatMessage('system', 'Harici tarayıcıda açılamadı.');
            }
        });
    }

    // Devtools toggle (Cmd+Alt+I)
    document.addEventListener('keydown', async (e) => {
        if (e.metaKey && e.altKey && e.key.toLowerCase() === 'i') {
            try {
                if (window.__TAURI__?.webview?.internalToggleDevtools) {
                    await window.__TAURI__.webview.internalToggleDevtools();
                } else if (tauriInvoke) {
                    // Tauri JS API yoksa main process üzerinden tetiklenebilir (opsiyonel)
                    await tauriInvoke('open_or_navigate_browser', { url: state.currentUrl || 'about:blank' });
                }
            } catch (err) {
                console.error('Devtools toggle hatası:', err);
            }
        }
    });
    
    // Chat events
    messageInput.addEventListener('keypress', (e) => {
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
    
    // Model selection
    modelSelect.addEventListener('change', (e) => {
        state.currentModel = e.target.value;
        updateChatStatus(`Model: ${state.currentModel}`);
    });
    
    // Auto-resize textarea
    messageInput.addEventListener('input', () => {
        messageInput.style.height = 'auto';
        messageInput.style.height = Math.min(messageInput.scrollHeight, 100) + 'px';
    });

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

    // Tabs events
    if (newTabBtn) newTabBtn.addEventListener('click', () => createNewTab());
    document.addEventListener('keydown', (e) => {
        if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 't') {
            e.preventDefault();
            createNewTab();
        }
    });
}

async function loadOllamaModels() {
    if (!tauriInvoke) {
        console.error('Tauri invoke fonksiyonu bulunamadı');
        return;
    }
    
    try {
        console.log('=== OLLAMA MODEL YÜKLEME BAŞLIYOR ===');
        
        showLoading('Ollama modelleri yükleniyor...');
        updateChatStatus('Modeller yükleniyor...', 'processing');
        
        console.log('Tauri invoke çağrılıyor: get_ollama_models');
        
        const models = await tauriInvoke('get_ollama_models');
        
        console.log('Modeller yüklendi:', models);
        state.availableModels = models;
        
        // Model select'i doldur
        modelSelect.innerHTML = '';
        
        if (!models || models.length === 0) {
            modelSelect.innerHTML = '<option value="">Model bulunamadı</option>';
            updateChatStatus('Model bulunamadı - Ollama çalışıyor mu?', 'error');
        } else {
            models.forEach((model) => {
                const option = document.createElement('option');
                option.value = model.name;
                option.textContent = model.name;
                modelSelect.appendChild(option);
            });
            
            // İlk modeli seçcargo
            
            state.currentModel = models[0].name;
            modelSelect.value = state.currentModel;
            updateChatStatus(`Model: ${state.currentModel}`);
        }
        
    } catch (error) {
        console.error('Ollama modelleri yüklenirken hata:', error);
        modelSelect.innerHTML = '<option value="">Model yüklenemedi</option>';
        updateChatStatus(`Hata: ${error.message || error}`, 'error');
    } finally {
        hideLoading();
    }
}

// Settings helpers
let settingsBtn, settingsModal, settingsClose, settingsSave, ollamaUrlInput;

async function openSettings() {
    // Get current url from backend
    try {
        if (tauriInvoke) {
            const current = await tauriInvoke('get_ollama_base_url');
            if (ollamaUrlInput) ollamaUrlInput.value = current || 'http://localhost:11434';
        }
    } catch (e) {
        console.error('Ayarlar okunamadı:', e);
    }
    settingsModal.classList.remove('hidden');
}

function closeSettings() {
    settingsModal.classList.add('hidden');
}

async function saveSettings() {
    const value = (ollamaUrlInput?.value || '').trim();
    if (!value) return;
    try {
        if (tauriInvoke) {
            await tauriInvoke('set_ollama_base_url', { value });
            // Modelleri yeni URL'e göre yeniden yükle
            await loadOllamaModels();
            updateChatStatus('Ayarlar kaydedildi');
        }
        closeSettings();
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

async function loadUrl(url) {
    // URL'yi düzelt
    if (!url.startsWith('http://') && !url.startsWith('https://')) {
        url = 'https://' + url;
    }
    
    state.currentUrl = url;
    urlInput.value = url;
    
    updateStatus('loading', 'Yükleniyor...');
    currentUrlSpan.textContent = url;
    
    // Hide welcome screen and show browser
    hideWelcomeScreen();
    
    // Tüm siteler için ayrı WebviewWindow'da aç (Chromium/WebKit)
    try {
        if (tauriInvoke) {
            ensureActiveTab();
            await tauriInvoke('open_or_navigate_browser_tab', { tabId: state.activeTabId, url });
            await tauriInvoke('show_only_tab', { tabId: state.activeTabId });
            updateStatus('ready', 'Sayfa yüklendi');
        } else {
            // Fallback
            window.open(url, '_blank');
            updateStatus('ready', 'Tarayıcıda açıldı');
        }
    } catch (err) {
        console.error('URL açma hatası:', err);
        updateStatus('error', 'Yükleme hatası');
        addChatMessage('system', 'URL yüklenirken hata oluştu');
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
    
    // Add user message to chat
    addChatMessage('user', message);
    messageInput.value = '';
    messageInput.style.height = 'auto';
    
    // Disable send button
    sendButton.disabled = true;
    updateChatStatus('Düşünüyorum...', 'processing');
    
    try {
        showLoading('Sayfa analiz ediliyor...');
        
        const response = await tauriInvoke('ask_question', {
            url: state.currentUrl,
            question: message,
            model: state.currentModel
        });
        
        addChatMessage('bot', response);
        updateChatStatus(`Model: ${state.currentModel}`);
        
    } catch (error) {
        console.error('Soru sorulurken hata:', error);
        addChatMessage('system', `Hata: ${error.message || error}`);
        updateChatStatus('Hata oluştu', 'error');
    } finally {
        sendButton.disabled = false;
        hideLoading();
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
}

function updateStatus(type, message) {
    statusIndicator.className = `status-${type}`;
    statusIndicator.textContent = message;
}

function updateChatStatus(message, type = '') {
    const statusText = document.querySelector('.status-text');
    if (statusText) {
        statusText.textContent = message;
        statusText.className = `status-text ${type}`;
    }
}

function showWelcomeScreen() {
    welcomeScreen.style.display = 'flex';
    browserFrame.style.display = 'none';
    currentUrlSpan.textContent = 'Hoş geldiniz';
    updateStatus('ready', 'Hazır');
}

function hideWelcomeScreen() {
    welcomeScreen.style.display = 'none';
    browserFrame.style.display = 'block';
}

function showLoading(message = 'Yükleniyor...') {
    if (loadingOverlay) {
        loadingOverlay.querySelector('.loading-text').textContent = message;
        loadingOverlay.classList.add('show');
    }
    state.isLoading = true;
}

function hideLoading() {
    if (loadingOverlay) {
        loadingOverlay.classList.remove('show');
    }
    state.isLoading = false;
}

// Quick links function (called from HTML)
window.loadUrl = loadUrl;

// Tabs helpers
function createNewTab() {
    const id = 'tab-' + Math.random().toString(36).slice(2, 8);
    state.tabs.push({ id, title: 'Yeni Sekme', url: '' });
    state.activeTabId = id;
    renderTabs();
}

function switchTab(id) {
    state.activeTabId = id;
    const tab = state.tabs.find(t => t.id === id);
    state.currentUrl = tab?.url || '';
    urlInput.value = state.currentUrl;
    currentUrlSpan.textContent = state.currentUrl;
    renderTabs();
    if (tauriInvoke && id) {
        tauriInvoke('show_only_tab', { tabId: id }).catch(()=>{});
    }
}

function closeTab(id) {
    const idx = state.tabs.findIndex(t => t.id === id);
    if (idx === -1) return;
    const wasActive = state.activeTabId === id;
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
        el.innerHTML = `<span class="tab-title">${tab.title}</span> <button class="tab-close" title="Kapat">×</button>`;
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

// Log Tauri environment on load
console.log('=== TAURI ENVIRONMENT CHECK ===');
console.log('window.__TAURI__:', window.__TAURI__);
console.log('window.__TAURI_INTERNALS__:', window.__TAURI_INTERNALS__);
console.log('================================');