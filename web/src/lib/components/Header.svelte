<script lang="ts">
    import { page } from '$app/state';
    import { onMount } from 'svelte';

    interface Props {
        onSearchClick?: () => void;
    }

    let { onSearchClick }: Props = $props();

    let scrolled = $state(false);
    let mobileMenuOpen = $state(false);

    const navItems = [
        { href: '/', label: 'Home' },
        { href: '/movies', label: 'Movies' },
        { href: '/shows', label: 'Shows' },
        { href: '/collections', label: 'Collections' }
    ];

    function handleScroll() {
        scrolled = window.scrollY > 50;
    }

    onMount(() => {
        window.addEventListener('scroll', handleScroll, { passive: true });
        return () => window.removeEventListener('scroll', handleScroll);
    });

    function isActive(href: string): boolean {
        if (href === '/') {
            return page.url.pathname === '/';
        }
        return page.url.pathname.startsWith(href);
    }
</script>

<header
    class="fixed top-0 left-0 right-0 z-[300] transition-all duration-300 {scrolled ? 'bg-[#141414]' : 'bg-gradient-to-b from-black/80 to-transparent'}"
>
    <nav class="flex items-center justify-between px-4 md:px-12 py-4" aria-label="Main navigation">
        <!-- Logo -->
        <a href="/" class="flex items-center space-x-2 text-red-600 font-black text-2xl md:text-3xl tracking-tight">
            <span>HOMEFLIX</span>
        </a>

        <!-- Desktop Navigation -->
        <ul class="hidden md:flex items-center space-x-6">
            {#each navItems as item}
                <li>
                    <a
                        href={item.href}
                        class="text-sm font-medium transition-colors {isActive(item.href) ? 'text-white' : 'text-gray-300 hover:text-white'}"
                        aria-current={isActive(item.href) ? 'page' : undefined}
                    >
                        {item.label}
                    </a>
                </li>
            {/each}
        </ul>

        <!-- Right side actions -->
        <div class="flex items-center space-x-4">
            <!-- Search button -->
            <button
                aria-label="Search"
                class="text-white hover:text-gray-300 transition-colors p-2"
                onclick={() => onSearchClick?.()}
            >
                <svg aria-hidden="true" xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
                </svg>
            </button>

            <!-- Mobile menu button -->
            <button
                class="md:hidden text-white p-2"
                aria-label={mobileMenuOpen ? 'Close menu' : 'Open menu'}
                aria-expanded={mobileMenuOpen}
                onclick={() => mobileMenuOpen = !mobileMenuOpen}
            >
                {#if mobileMenuOpen}
                    <svg aria-hidden="true" xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                    </svg>
                {:else}
                    <svg aria-hidden="true" xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16M4 18h16" />
                    </svg>
                {/if}
            </button>
        </div>
    </nav>

    <!-- Mobile Navigation Menu -->
    {#if mobileMenuOpen}
        <div class="md:hidden bg-[#141414] border-t border-gray-800">
            <ul class="flex flex-col py-4">
                {#each navItems as item}
                    <li>
                        <a
                            href={item.href}
                            class="block px-6 py-3 text-base font-medium transition-colors {isActive(item.href) ? 'text-white bg-gray-800' : 'text-gray-300 hover:text-white hover:bg-gray-800/50'}"
                            aria-current={isActive(item.href) ? 'page' : undefined}
                            onclick={() => mobileMenuOpen = false}
                        >
                            {item.label}
                        </a>
                    </li>
                {/each}
            </ul>
        </div>
    {/if}
</header>

<!-- Spacer to prevent content from going under fixed header -->
<div class="h-0" aria-hidden="true"></div>
