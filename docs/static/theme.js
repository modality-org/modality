// Theme Toggle
(function() {
    // Get stored theme or default to system preference
    function getPreferredTheme() {
        const stored = localStorage.getItem('theme');
        if (stored) return stored;
        return window.matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark';
    }

    // Apply theme
    function setTheme(theme) {
        document.documentElement.setAttribute('data-theme', theme);
        localStorage.setItem('theme', theme);
    }

    // Toggle theme
    function toggleTheme() {
        const current = document.documentElement.getAttribute('data-theme') || 'dark';
        setTheme(current === 'dark' ? 'light' : 'dark');
    }

    // Initialize on load
    setTheme(getPreferredTheme());

    // Expose toggle function
    window.toggleTheme = toggleTheme;

    // Mobile menu toggle
    window.toggleMenu = function() {
        document.querySelector('.sidebar').classList.toggle('open');
        document.querySelector('.sidebar-overlay').classList.toggle('active');
    };

    // Close menu when clicking overlay
    document.addEventListener('DOMContentLoaded', function() {
        const overlay = document.querySelector('.sidebar-overlay');
        if (overlay) {
            overlay.addEventListener('click', function() {
                document.querySelector('.sidebar').classList.remove('open');
                overlay.classList.remove('active');
            });
        }
    });
})();
