const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('lingetShell', {
  // TODO: Add real read/query/update APIs here instead of only exposing window controls.
  close: () => ipcRenderer.invoke('window:close'),
});
