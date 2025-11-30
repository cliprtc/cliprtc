export default {
  public: {
    button: {
      save: '保存',
      cancel: '取消',
      confirm: '确认',
    },
  },
  tray: {
    menu: {
      open_settings: '打开设置',
      version: '版本',
      restart: '重启程序',
      quit: '退出',
    },
  },
  window: {
    settings: {
      title: '设置',
      general: {
        label: '常规',
        subtitle: '应用程序',
        key: {
          name: '密钥',
          tips: '出于安全原因，只有使用相同密钥的用户才能访问。',
          alert: {
            title: '保存新密钥？',
            tips: '更改密钥需要重启应用才能生效。确定要继续吗？',
          },
        },
        autoStart: {
          name: '开机自启',
          tips: '在系统启动时自动启动应用。',
        },
        switchLanguage: {
          name: '切换语言',
          tips: '将应用语言更改为您偏好的语言。',
        },
      },
      about: {
        label: '关于',
        subtitle: '关于应用程序',
        version: '版本',
        source: {
          label: '开源仓库',
          issue: '报告问题',
        },
      },
    },
  },
}
