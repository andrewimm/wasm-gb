// This code is intentionally left unminified for your perusal
(function() {

function openDB(name, version) {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(name, version);
    request.onerror = function(e) {
      reject(e);
    };
    request.onsuccess = function(e) {
      resolve(e.target.result);
    };
    request.onupgradeneeded = function(e) {
      const db = e.target.result;
      const store = db.createObjectStore('savestate', {keyPath: 'name'});
      store.transaction.oncomplete = function(ev) {
        resolve(db);
      };
    };
  });
}

function writeSaveState(db, name, sram) {
  return new Promise((resolve, reject) => {
    const transaction = db.transaction(['savestate'], 'readwrite');
    const store = transaction.objectStore('savestate');
    const update = store.put({
      name: name,
      sram: sram,
    });
    update.onsuccess = function(e) {
      resolve();
    };
    update.onerror = function(e) {
      reject();
    };
  });
}

function getSaveState(db, name) {
  return new Promise((resolve, reject) => {
    const transaction = db.transaction(['savestate'], 'readwrite');
    const store = transaction.objectStore('savestate');
    const request = store.get(name);
    request.onsuccess = function(e) {
      const data = e.target.result;
      if (data) {
        resolve(data.sram);
      } else {
        reject();
      }
    };
    request.onerror = function(e) {
      reject(e);
    }
  });
}

class SaveState {
  constructor(vm) {
    this.vm = vm;
    this.pendingOpen = openDB('gb', 1);
  }

  getCartName() {
    const nameBytes = [];
    for (let i = 0; i < 16; i++) {
      nameBytes[i] = String.fromCharCode(this.vm.mem.rom[0x134 + i]);
    }
    return nameBytes.join('');
  }

  getRAMSize() {
    switch (this.vm.mem.rom[0x149]) {
      case 1:
        return 2 * 1024;
      case 2:
        return 8 * 1024;
      case 3:
        return 32 * 1024;
      case 4:
        return 128 * 1024;
    }
    return 0;
  }

  getRAMBuffer(size) {
    if (this._buffer) {
      return this._buffer;
    }
    this._buffer = new ArrayBuffer(size);
    return this._buffer;
  }

  save() {
    const name = this.getCartName();
    const size = this.getRAMSize();
    const ramBuffer = this.getRAMBuffer(size);
    const ramWindow = new Uint8Array(ramBuffer, 0, size);
    for (let i = 0; i < size; i++) {
      ramWindow[i] = this.vm.mem.ram[i];
    }
    return this.pendingOpen.then(db => {
      return writeSaveState(db, name, ramBuffer);
    });
  }

  load() {
    const name = this.getCartName();
    return this.pendingOpen.then(db => {
      return getSaveState(db, name);
    }).then(ramBuffer => {
      const ramWindow = new Uint8Array(ramBuffer, 0, ramBuffer.byteLength);
      for (let i = 0; i < ramBuffer.byteLength; i++) {
        this.vm.mem.ram[i] = ramWindow[i];
      }
    });
  }
}

window.SaveState = SaveState;
})();