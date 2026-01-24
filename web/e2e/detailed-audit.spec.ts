import { test, expect } from '@playwright/test';

const baseUrl = process.env.PLAYWRIGHT_BASE_URL ?? 'http://localhost:5173';

test.describe('Detailed UX/A11y Audit', () => {
  test('comprehensive accessibility and UX analysis', async ({ page }) => {
    await page.goto(`${baseUrl}/`);
    await page.waitForLoadState('domcontentloaded');
    await page.waitForTimeout(1000);

    // 1. Check semantic HTML structure
    const semanticStructure = await page.evaluate(() => {
      return {
        hasDoctype: document.doctype !== null,
        htmlLang: document.documentElement.lang,
        hasMain: !!document.querySelector('main'),
        hasNav: !!document.querySelector('nav'),
        hasHeader: !!document.querySelector('header'),
        hasFooter: !!document.querySelector('footer'),
        headingStructure: Array.from(document.querySelectorAll('h1, h2, h3, h4, h5, h6')).map(h => ({
          tag: h.tagName,
          text: h.textContent?.trim()
        })),
        skipLink: !!document.querySelector('a[href^="#"]')
      };
    });

    console.log('\n=== SEMANTIC STRUCTURE ===');
    console.log(JSON.stringify(semanticStructure, null, 2));

    // 2. Keyboard navigation audit
    const keyboardNav = await page.evaluate(() => {
      const focusableElements = Array.from(document.querySelectorAll(
        'button, a, input, select, textarea, [tabindex]:not([tabindex="-1"])'
      ));

      return {
        totalFocusable: focusableElements.length,
        buttons: focusableElements.filter(el => el.tagName === 'BUTTON').map(el => ({
          text: el.textContent?.trim(),
          ariaLabel: el.getAttribute('aria-label'),
          disabled: (el as HTMLButtonElement).disabled,
          type: (el as HTMLButtonElement).type
        })),
        links: focusableElements.filter(el => el.tagName === 'A').map(el => ({
          text: el.textContent?.trim(),
          href: (el as HTMLAnchorElement).href,
          ariaLabel: el.getAttribute('aria-label')
        }))
      };
    });

    console.log('\n=== KEYBOARD NAVIGATION ===');
    console.log(JSON.stringify(keyboardNav, null, 2));

    // 3. Test actual keyboard navigation
    const tabSequence = [];
    for (let i = 0; i < 10; i++) {
      await page.keyboard.press('Tab');
      await page.waitForTimeout(100);
      const focused = await page.evaluate(() => {
        const el = document.activeElement;
        return {
          tag: el?.tagName,
          text: el?.textContent?.trim().substring(0, 30),
          ariaLabel: el?.getAttribute('aria-label'),
          role: el?.getAttribute('role'),
          href: el?.tagName === 'A' ? (el as HTMLAnchorElement).href : null
        };
      });
      tabSequence.push(focused);
    }

    console.log('\n=== TAB SEQUENCE (first 10) ===');
    console.log(JSON.stringify(tabSequence, null, 2));

    // 4. Check ARIA attributes and roles
    const ariaAudit = await page.evaluate(() => {
      const elementsWithAria = Array.from(document.querySelectorAll('[role], [aria-label], [aria-labelledby], [aria-describedby]'));

      return {
        totalAriaElements: elementsWithAria.length,
        elements: elementsWithAria.map(el => ({
          tag: el.tagName,
          role: el.getAttribute('role'),
          ariaLabel: el.getAttribute('aria-label'),
          ariaLabelledBy: el.getAttribute('aria-labelledby'),
          ariaDescribedBy: el.getAttribute('aria-describedby')
        }))
      };
    });

    console.log('\n=== ARIA AUDIT ===');
    console.log(JSON.stringify(ariaAudit, null, 2));

    // 5. Image accessibility
    const imageAudit = await page.evaluate(() => {
      const images = Array.from(document.querySelectorAll('img'));
      return {
        total: images.length,
        withoutAlt: images.filter(img => !img.hasAttribute('alt')).length,
        withEmptyAlt: images.filter(img => img.alt === '').length,
        decorativeImages: images.filter(img => img.alt === '').length,
        images: images.map(img => ({
          src: img.src.substring(0, 80),
          alt: img.alt,
          loading: img.loading,
          width: img.width,
          height: img.height
        }))
      };
    });

    console.log('\n=== IMAGE ACCESSIBILITY ===');
    console.log(JSON.stringify(imageAudit, null, 2));

    // 6. Color contrast check (sampling)
    const contrastCheck = await page.evaluate(() => {
      // Helper function to calculate relative luminance
      const getLuminance = (rgb: number[]) => {
        const [r, g, b] = rgb.map(val => {
          val = val / 255;
          return val <= 0.03928 ? val / 12.92 : Math.pow((val + 0.055) / 1.055, 2.4);
        });
        return 0.2126 * r + 0.7152 * g + 0.0722 * b;
      };

      const parseColor = (color: string): number[] => {
        const temp = document.createElement('div');
        temp.style.color = color;
        document.body.appendChild(temp);
        const computed = window.getComputedStyle(temp).color;
        document.body.removeChild(temp);
        const match = computed.match(/\d+/g);
        return match ? match.map(Number) : [0, 0, 0];
      };

      const getContrast = (fg: string, bg: string): number => {
        const fgLum = getLuminance(parseColor(fg));
        const bgLum = getLuminance(parseColor(bg));
        const lighter = Math.max(fgLum, bgLum);
        const darker = Math.min(fgLum, bgLum);
        return (lighter + 0.05) / (darker + 0.05);
      };

      const textElements = Array.from(document.querySelectorAll('h1, h2, h3, p, button, a, span')).slice(0, 20);

      return textElements.map(el => {
        const styles = window.getComputedStyle(el);
        const fg = styles.color;
        const bg = styles.backgroundColor;
        const fontSize = parseFloat(styles.fontSize);
        const fontWeight = styles.fontWeight;
        const isLargeText = fontSize >= 18 || (fontSize >= 14 && parseInt(fontWeight) >= 700);

        let contrast = 0;
        try {
          contrast = getContrast(fg, bg);
        } catch (e) {
          contrast = 0;
        }

        const requiredRatio = isLargeText ? 3 : 4.5;
        const passes = contrast >= requiredRatio;

        return {
          tag: el.tagName,
          text: el.textContent?.trim().substring(0, 40),
          fg,
          bg,
          fontSize: `${fontSize}px`,
          contrast: contrast.toFixed(2),
          required: requiredRatio,
          passes
        };
      });
    });

    console.log('\n=== COLOR CONTRAST (sample) ===');
    console.log(JSON.stringify(contrastCheck, null, 2));

    // 7. Interactive element sizing (touch targets)
    const touchTargets = await page.evaluate(() => {
      const interactive = Array.from(document.querySelectorAll('button, a, input, select'));

      return interactive.map(el => {
        const rect = el.getBoundingClientRect();
        const meetsMinimum = rect.width >= 44 && rect.height >= 44;

        return {
          tag: el.tagName,
          text: el.textContent?.trim().substring(0, 30),
          width: Math.round(rect.width),
          height: Math.round(rect.height),
          meetsMinimum,
          visible: rect.width > 0 && rect.height > 0
        };
      }).filter(item => item.visible);
    });

    console.log('\n=== TOUCH TARGET SIZES ===');
    console.log(JSON.stringify(touchTargets, null, 2));

    // 8. Check for loading states and feedback
    const loadingStates = await page.evaluate(() => {
      return {
        hasLoadingIndicators: !!document.querySelector('[role="progressbar"], [aria-busy="true"]'),
        hasLiveRegions: !!document.querySelector('[aria-live]'),
        hasAlerts: !!document.querySelector('[role="alert"]')
      };
    });

    console.log('\n=== LOADING STATES ===');
    console.log(JSON.stringify(loadingStates, null, 2));

    // 9. Motion and animation preferences
    const motionCheck = await page.evaluate(() => {
      const prefersReducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
      const elementsWithTransition = Array.from(document.querySelectorAll('*')).filter(el => {
        const styles = window.getComputedStyle(el);
        return styles.transition !== 'all 0s ease 0s' && styles.transition !== 'none';
      }).length;

      return {
        prefersReducedMotion,
        elementsWithTransition
      };
    });

    console.log('\n=== MOTION/ANIMATION ===');
    console.log(JSON.stringify(motionCheck, null, 2));
  });
});
