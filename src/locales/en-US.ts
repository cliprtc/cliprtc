export default {
  public: {
    button: {
      save: 'Save',
      cancel: 'Cancel',
      confirm: 'Confirm',
    },
  },
  tray: {
    menu: {
      open_settings: 'Open Settings',
      version: 'Version',
      restart: 'Restart App',
      quit: 'Quit',
    },
  },
  window: {
    settings: {
      title: 'Settings',
      general: {
        label: 'General',
        subtitle: 'Application',
        key: {
          name: 'Key',
          tips: 'For security reasons, only users with the same key can access it.',
          alert: {
            title: 'Save new key?',
            tips: 'Changing the key requires restarting the application to take effect. Are you sure you want to continue?',
          },
        },
        autoStart: {
          name: 'Auto Start',
          tips: 'Automatically launch the app on system startup.',
        },
        switchLanguage: {
          name: 'Switch Language',
          tips: 'Change the app language to your preferred one.',
        },
      },
      about: {
        label: 'About',
        subtitle: 'About the App',
        version: 'Version',
        source: {
          label: 'Open Source Repository',
          issue: 'Report an Issue',
        },
      },
    },
  },
}
