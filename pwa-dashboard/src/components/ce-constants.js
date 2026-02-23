// Classification des types de règles
export const EVENT_TYPES = ['mode_change', 'sensor_alert', 'agent_status', 'manual', 'plugin_health', 'scheduled']
export const STATE_TYPES = ['current_mode', 'time_range', 'day_of_week', 'day_of_month', 'month', 'sensor_value', 'agent_online']

export function isEventType(type) {
  return EVENT_TYPES.includes(type)
}

export function isStateType(type) {
  return STATE_TYPES.includes(type)
}
