/**
 * Registry des widgets dynamiques
 * 
 * Système de widgets pilotés par les manifestes des plugins
 * Chargement et instanciation automatique basés sur la configuration
 */

class WidgetRegistry {
  constructor() {
    this.widgets = new Map()
    this.loadedManifests = new Map()
  }
  
  /**
   * Enregistre un type de widget
   */
  registerWidget(type, widgetClass) {
    this.widgets.set(type, widgetClass)
    console.log(`🔧 Registered widget type: ${type}`)
  }
  
  /**
   * Charge les widgets depuis les manifestes des plugins
   */
  async loadWidgetsFromManifests(pluginManifests) {
    console.log('📦 Loading widgets from plugin manifests...')
    
    for (const manifest of pluginManifests) {
      await this.loadWidgetsFromManifest(manifest)
    }
  }
  
  /**
   * Charge les widgets d'un manifeste spécifique
   */
  async loadWidgetsFromManifest(manifest) {
    const widgets = manifest.widgets || []
    
    for (const widgetConfig of widgets) {
      try {
        await this.loadWidget(widgetConfig, manifest)
      } catch (error) {
        console.error(`❌ Failed to load widget from ${manifest.name}:`, error)
      }
    }
  }
  
  /**
   * Charge un widget individuel
   */
  async loadWidget(widgetConfig, pluginManifest) {
    const { type, component, config } = widgetConfig
    
    // Charger le composant dynamiquement si spécifié
    if (component) {
      try {
        await import(component)
        console.log(`📦 Loaded widget component: ${component}`)
      } catch (error) {
        console.warn(`⚠️ Failed to load widget component ${component}:`, error)
      }
    }
    
    // Enregistrer la configuration du widget
    this.loadedManifests.set(`${pluginManifest.name}:${type}`, {
      plugin: pluginManifest.name,
      type,
      config,
      manifest: pluginManifest
    })
  }
  
  /**
   * Crée une instance de widget
   */
  createWidget(type, config = {}) {
    const WidgetClass = this.widgets.get(type)
    
    if (!WidgetClass) {
      console.warn(`⚠️ Widget type not found: ${type}`)
      return null
    }
    
    const widget = new WidgetClass()
    
    // Appliquer la configuration
    Object.assign(widget, config)
    
    return widget
  }
  
  /**
   * Récupère tous les widgets disponibles
   */
  getAvailableWidgets() {
    return Array.from(this.widgets.keys())
  }
  
  /**
   * Récupère les widgets chargés depuis les manifestes
   */
  getLoadedWidgets() {
    return Array.from(this.loadedManifests.values())
  }
  
  /**
   * Crée tous les widgets pour un plugin donné
   */
  createWidgetsForPlugin(pluginName) {
    const widgets = []
    
    for (const [key, widgetManifest] of this.loadedManifests) {
      if (widgetManifest.plugin === pluginName) {
        const widget = this.createWidget(widgetManifest.type, widgetManifest.config)
        if (widget) {
          widgets.push({
            widget,
            config: widgetManifest
          })
        }
      }
    }
    
    return widgets
  }
}

// TEMPORARILY DISABLED - Widget registry causing HTMLElement construction errors
// Will be re-enabled once the initialization issues are resolved

// // Enregistrement des widgets de base - DOIT être avant l'instanciation
// import './system-health-widget.js'
// import './plugins-widget.js'
// import './hosts-widget.js'
// import './notes-widget.js'
// import './agents-network-widget.js'

// // Instance singleton du registry
// const widgetRegistry = new WidgetRegistry()

// // Enregistrement des widgets après import
// // Utilisation de setTimeout pour s'assurer que les custom elements sont définis
// setTimeout(() => {
//   widgetRegistry.registerWidget('system-health', customElements.get('system-health-widget'))
//   widgetRegistry.registerWidget('plugins-manager', customElements.get('plugins-widget'))
//   widgetRegistry.registerWidget('hosts-monitor', customElements.get('hosts-widget'))
//   widgetRegistry.registerWidget('notes-manager', customElements.get('notes-widget'))
//   widgetRegistry.registerWidget('agents-network', customElements.get('agents-network-widget'))
// }, 0)

// Create a dummy registry for now
const widgetRegistry = new WidgetRegistry()

export { widgetRegistry, WidgetRegistry }