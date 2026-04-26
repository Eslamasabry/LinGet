const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('lingetShell', {
  close: () => ipcRenderer.invoke('window:close'),
});
