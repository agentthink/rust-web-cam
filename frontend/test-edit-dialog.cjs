const { chromium } = require('@playwright/test');

(async () => {
  const browser = await chromium.launch({ headless: true });
  const context = await browser.newContext();
  const page = await context.newPage();
  
  page.on('console', msg => {
    console.log(`[Browser Console ${msg.type()}]`, msg.text());
  });

  page.on('response', response => {
    if (response.url().includes('/devices/') && response.request().method() === 'PUT') {
      console.log('[API PUT]', response.url());
      response.json().then(body => console.log('[API PUT Response]', JSON.stringify(body, null, 2))).catch(() => console.log('[API PUT Response] non-json'));
    }
  });
  
  try {
    console.log('1. Opening login page...');
    await page.goto('http://localhost:5173/login');
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(1000);
    
    console.log('2. Filling login form...');
    await page.fill('input[placeholder*="用户名"], input[placeholder*="username"]', 'admin');
    await page.fill('input[type="password"]', 'admin123');
    
    console.log('3. Clicking login button...');
    await page.click('button[type="submit"], button:has-text("登录")');
    
    await page.waitForURL('**/', { timeout: 10000 });
    console.log('4. Logged in');
    
    await page.waitForTimeout(2000);
    
    console.log('5. Clicking device management menu...');
    const deviceMenu = page.locator('text=设备管理').first();
    if (await deviceMenu.isVisible()) {
      await deviceMenu.click();
      await page.waitForTimeout(1000);
    }
    
    console.log('6. Clicking device list...');
    const deviceListMenu = page.locator('text=设备列表').first();
    if (await deviceListMenu.isVisible()) {
      await deviceListMenu.click();
      await page.waitForTimeout(2000);
    }
    
    console.log('7. Looking for device table...');
    await page.waitForSelector('.el-table', { timeout: 10000 });
    await page.waitForTimeout(1000);
    
    console.log('8. Looking for edit button...');
    const editButtons = await page.locator('button:has-text("编辑")').all();
    console.log(`   Found ${editButtons.length} edit buttons`);
    
    if (editButtons.length > 0) {
      console.log('9. Clicking first edit button...');
      await editButtons[0].click();
      
      await page.waitForSelector('.el-dialog', { state: 'visible', timeout: 5000 });
      await page.waitForTimeout(3000);
      
      console.log('10. Dialog is visible, getting SIP ID fields...');
      
      const allInputs = await page.locator('.el-dialog input').all();
      console.log('   All inputs in dialog:');
      for (let i = 0; i < allInputs.length; i++) {
        const input = allInputs[i];
        const value = await input.inputValue();
        const placeholder = await input.getAttribute('placeholder');
        const disabled = await input.getAttribute('disabled');
        console.log(`   Input ${i}: value="${value}", placeholder="${placeholder}", disabled=${disabled !== null}`);
      }

      console.log('11. Clicking save button...');
      const saveButton = page.locator('.el-dialog button:has-text("保存修改")');
      if (await saveButton.isVisible()) {
        await saveButton.click();
        await page.waitForTimeout(3000);
        console.log('12. Save clicked, checking result...');
      } else {
        console.log('   Save button not found');
      }
      
      await page.screenshot({ path: 'edit-dialog.png', fullPage: false });
      console.log('\n13. Screenshot saved to edit-dialog.png');
      
    } else {
      console.log('No edit buttons found. Current URL:', page.url());
    }
    
  } catch (error) {
    console.error('Error:', error.message);
    await page.screenshot({ path: 'error.png' });
  } finally {
    await browser.close();
  }
})();
