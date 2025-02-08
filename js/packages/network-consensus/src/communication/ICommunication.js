/**
 * @interface
 */
export class ICommunication {
  /**
   * @param {object} params
   * @param {string} params.from
   * @param {object} params.block_data
   * @returns {Promise<void>}
   */
  async broadcastDraftBlock({ from, block_data }) {}

  /**
   * @param {object} params
   * @param {string} params.from
   * @param {string} params.to
   * @param {object} params.ack_data
   * @returns {Promise<void>}
   */
  async sendBlockAck({ from, to, ack_data }) {}

  /**
   * @param {object} params
   * @param {string} params.from
   * @param {string} params.to
   * @param {object} params.ack_data
   * @returns {Promise<void>}
   */
  async sendBlockLateAck({ from, to, ack_data }) {}

  /**
   * @param {object} params
   * @param {string} params.from
   * @param {object} params.block_data
   * @returns {Promise<void>}
   */
  async broadcastCertifiedBlock({ from, block_data }) {}

  /**
   * @param {object} params
   * @param {string} params.from
   * @param {string} params.to
   * @param {string} params.scribe
   * @param {number} params.round
   * @returns {Promise<object|undefined>}
   */
  async fetchScribeRoundCertifiedBlock({ from, to, scribe, round }) {}
}
