import { test } from '@playwright/test';

test.describe('Responsive Design Audit', () => {
const baseUrl = process.env.PLAYWRIGHT_BASE_URL ?? 'http://localhost:5173';

const viewports = [
    { name: 'Mobile Small', width: 320, height: 568 },
    { name: 'Mobile Medium', width: 375, height: 812 },
    { name: 'Mobile Large', width: 414, height: 896 },
    { name: 'Tablet Portrait', width: 768, height: 1024 },
    { name: 'Tablet Landscape', width: 1024, height: 768 },
    { name: 'Desktop Small', width: 1280, height: 720 },
    { name: 'Desktop Medium', width: 1440, height: 900 },
    { name: 'Desktop Large', width: 1920, height: 1080 }
  ];

  for (const viewport of viewports) {
    test(`${viewport.name} (${viewport.width}x${viewport.height})`, async ({ page }) => {
      await page.setViewportSize({ width: viewport.width, height: viewport.height });
      await page.goto(`${baseUrl}/`);
      await page.waitForLoadState('domcontentloaded');
      await page.waitForTimeout(500);

      // Check for horizontal scrolling
      const hasHorizontalScroll = await page.evaluate(() => {
        return document.documentElement.scrollWidth > document.documentElement.clientWidth;
      });

      // Check button sizes and touch targets
      const touchTargetInfo = await page.evaluate(() => {
        const buttons = Array.from(document.querySelectorAll('button'));
        const tooSmall = buttons.filter(btn => {
          const rect = btn.getBoundingClientRect();
          return (rect.width < 44 || rect.height < 44) && rect.width > 0;
        });

        return {
          totalButtons: buttons.length,
          tooSmallCount: tooSmall.length,
          tooSmall: tooSmall.map(btn => ({
            text: btn.textContent?.trim() || btn.getAttribute('aria-label'),
            width: Math.round(btn.getBoundingClientRect().width),
            height: Math.round(btn.getBoundingClientRect().height)
          }))
        };
      });

      // Check text overflow
      const textOverflow = await page.evaluate(() => {
        const elements = Array.from(document.querySelectorAll('h1, h2, h3, p'));
        const overflowing = elements.filter(el => {
          return el.scrollWidth > el.clientWidth;
        });

        return {
          totalTextElements: elements.length,
          overflowingCount: overflowing.length
        };
      });

      // Check if hero content is visible
      const heroVisibility = await page.evaluate(() => {
        const hero = document.querySelector('h1');
        if (!hero) return { visible: false };

        const rect = hero.getBoundingClientRect();
        return {
          visible: rect.top >= 0 && rect.bottom <= window.innerHeight,
          top: rect.top,
          bottom: rect.bottom,
          viewportHeight: window.innerHeight
        };
      });

      console.log(`\n=== ${viewport.name} (${viewport.width}x${viewport.height}) ===`);
      console.log('Horizontal scroll:', hasHorizontalScroll ? 'YES (ISSUE)' : 'NO (GOOD)');
      console.log('Touch targets:', JSON.stringify(touchTargetInfo, null, 2));
      console.log('Text overflow:', JSON.stringify(textOverflow, null, 2));
      console.log('Hero visibility:', JSON.stringify(heroVisibility, null, 2));

      // Take screenshot
      await page.screenshot({
        path: `/tmp/responsive-${viewport.width}x${viewport.height}.png`,
        fullPage: false
      });
    });
  }

  test('Focus visibility', async ({ page }) => {
    await page.setViewportSize({ width: 1920, height: 1080 });
    await page.goto(`${baseUrl}/`);
    await page.waitForLoadState('domcontentloaded');

    // Test focus indicators
    await page.keyboard.press('Tab');
    await page.waitForTimeout(200);

    const focusIndicator = await page.evaluate(() => {
      const el = document.activeElement;
      if (!el) return null;

      const styles = window.getComputedStyle(el);
      return {
        element: el.tagName,
        outline: styles.outline,
        outlineWidth: styles.outlineWidth,
        outlineStyle: styles.outlineStyle,
        outlineColor: styles.outlineColor,
        boxShadow: styles.boxShadow,
        border: styles.border
      };
    });

    console.log('\n=== FOCUS INDICATOR ===');
    console.log(JSON.stringify(focusIndicator, null, 2));

    // Take screenshot with focus
    await page.screenshot({ path: '/tmp/focus-state.png' });
  });
});
