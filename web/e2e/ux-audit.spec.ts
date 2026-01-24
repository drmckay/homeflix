import { test, expect } from '@playwright/test';

const baseUrl = process.env.PLAYWRIGHT_BASE_URL ?? 'http://localhost:5173';

test.describe('UX Audit - Homeflix', () => {
  test('capture screenshots and analyze accessibility', async ({ page }) => {
    // Desktop viewport
    await page.setViewportSize({ width: 1920, height: 1080 });
    await page.goto(`${baseUrl}/`);
    await page.waitForLoadState('networkidle');

    // Capture desktop screenshot
    await page.screenshot({ path: '/tmp/homeflix-desktop.png', fullPage: true });

    // Capture accessibility info
    const a11yInfo = await page.evaluate(() => {
      const issues = [];

      // Check headings
      const headings = Array.from(document.querySelectorAll('h1, h2, h3, h4, h5, h6'));

      // Check buttons
      const buttons = Array.from(document.querySelectorAll('button'));
      const buttonsInfo = buttons.map(btn => ({
        text: btn.textContent?.trim(),
        hasAriaLabel: btn.hasAttribute('aria-label'),
        ariaLabel: btn.getAttribute('aria-label'),
        visible: btn.offsetParent !== null
      }));

      // Check images
      const images = Array.from(document.querySelectorAll('img'));
      const imagesInfo = images.map(img => ({
        src: img.src,
        alt: img.alt,
        hasAlt: img.hasAttribute('alt'),
        visible: img.offsetParent !== null
      }));

      // Check semantic structure
      const hasNav = !!document.querySelector('nav');
      const hasMain = !!document.querySelector('main');
      const hasHeader = !!document.querySelector('header');

      return {
        headings: headings.map(h => ({ tag: h.tagName, text: h.textContent?.trim() })),
        buttons: buttonsInfo,
        images: imagesInfo,
        semanticStructure: { hasNav, hasMain, hasHeader },
        totalInteractiveElements: buttons.length + document.querySelectorAll('a').length
      };
    });

    console.log('Accessibility Info:', JSON.stringify(a11yInfo, null, 2));

    // Test keyboard navigation
    await page.keyboard.press('Tab');
    await page.waitForTimeout(300);
    const firstFocus = await page.evaluate(() => {
      const el = document.activeElement;
      return {
        tag: el?.tagName,
        text: el?.textContent?.trim().substring(0, 50),
        ariaLabel: el?.getAttribute('aria-label')
      };
    });
    console.log('First focusable:', firstFocus);

    // Mobile viewport
    await page.setViewportSize({ width: 375, height: 812 });
    await page.goto(`${baseUrl}/`);
    await page.waitForLoadState('networkidle');
    await page.screenshot({ path: '/tmp/homeflix-mobile.png', fullPage: true });

    // Tablet viewport
    await page.setViewportSize({ width: 768, height: 1024 });
    await page.goto(`${baseUrl}/`);
    await page.waitForLoadState('networkidle');
    await page.screenshot({ path: '/tmp/homeflix-tablet.png', fullPage: true });

    // Back to desktop for interaction tests
    await page.setViewportSize({ width: 1920, height: 1080 });
    await page.goto(`${baseUrl}/`);
    await page.waitForLoadState('networkidle');

    // Test movie card hover
    const movieCard = page.locator('.group\\/card').first();
    if (await movieCard.count() > 0) {
      await movieCard.hover();
      await page.waitForTimeout(500);
      await page.screenshot({ path: '/tmp/homeflix-hover-card.png' });
    }

    // Test scroll buttons
    const scrollRight = page.locator('button[aria-label="Scroll right"]').first();
    if (await scrollRight.count() > 0) {
      const movieRowContainer = page.locator('.group\\/row').first();
      await movieRowContainer.hover();
      await page.waitForTimeout(300);
      await page.screenshot({ path: '/tmp/homeflix-scroll-buttons.png' });
    }
  });
});
