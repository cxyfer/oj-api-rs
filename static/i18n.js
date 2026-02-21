(function() {
    'use strict';

    var i18n = {
        currentLang: localStorage.getItem('admin_lang') || 'en',
        translations: {},

        /**
         * 根據路徑取得翻譯值 (例如: 'nav.dashboard')
         */
        t: function(key) {
            var keys = key.split('.');
            var value = this.translations;
            for (var i = 0; i < keys.length; i++) {
                if (value && value[keys[i]]) {
                    value = value[keys[i]];
                } else {
                    return key; // 找不到則返回原始 key
                }
            }
            return value;
        },

        /**
         * 載入指定語言的 JSON
         */
        loadTranslations: function(lang) {
            var self = this;
            return fetch('/static/i18n/' + lang + '.json')
                .then(function(response) {
                    if (!response.ok) throw new Error('Failed to load translations');
                    return response.json();
                })
                .then(function(data) {
                    self.translations = data;
                    self.currentLang = lang;
                    localStorage.setItem('admin_lang', lang);
                    self.applyTranslations();
                })
                .catch(function(err) {
                    console.error('i18n Error:', err);
                });
        },

        /**
         * 更新所有具有 data-i18n 屬性的元素
         */
        applyTranslations: function() {
            var elements = document.querySelectorAll('[data-i18n]');
            for (var i = 0; i < elements.length; i++) {
                var el = elements[i];
                var key = el.getAttribute('data-i18n');
                el.textContent = this.t(key);
            }

            var placeholders = document.querySelectorAll('[data-i18n-placeholder]');
            for (var j = 0; j < placeholders.length; j++) {
                var p = placeholders[j];
                var pKey = p.getAttribute('data-i18n-placeholder');
                p.setAttribute('placeholder', this.t(pKey));
            }

            // 更新 HTML lang 屬性
            document.documentElement.lang = this.currentLang;
            
            // 觸發自定義事件，讓其他模組知道語系已變更
            document.dispatchEvent(new CustomEvent('languageChanged', { detail: this.currentLang }));
        },

        /**
         * 切換語言
         */
        setLanguage: function(lang) {
            this.loadTranslations(lang);
        },

        init: function() {
            var self = this;
            document.addEventListener('DOMContentLoaded', function() {
                self.loadTranslations(self.currentLang);
            });
        }
    };

    // 暴露到全域
    window.i18n = i18n;
    i18n.init();
})();
