'use strict'

const IPFS = require('ipfs')
const OrbitDB = require('orbit-db')

const wrtc = require('wrtc') // or 'electron-webrtc'
const WebRTCStar = require('libp2p-webrtc-star')

//const databaseAddress = "/orbitdb/zdpuB3KuktCyvSaNTc6JmeTN9MLRYmBAfcFQaSw89NvoVgGiz/vip-list"
//const database = "vip-list"

async function main (database) {

    try {
        console.log("Connecting to IPFS...")
        const ipfs = await IPFS.create({
            config: {
                Addresses: {
                    Swarm: [
                        // "/ip4/0.0.0.0/tcp/4002",
                        // "/ip4/127.0.0.1/tcp/4003/ws",
                        "/dns4/wrtc-star1.par.dwebops.pub/tcp/443/wss/p2p-webrtc-star",
                        "/dns4/wrtc-star2.sjc.dwebops.pub/tcp/443/wss/p2p-webrtc-star",
                        '/dns4/webrtc-star.discovery.libp2p.io/tcp/443/wss/p2p-webrtc-star/'
                    ]
                }
            },
            libp2p: {
                modules: {
                    transport: [WebRTCStar]
                },
                config: {
                    peerDiscovery: {
                        webRTCStar: { // <- note the lower-case w - see https://github.com/libp2p/js-libp2p/issues/576
                            enabled: true
                        }
                    },
                    transport: {
                        WebRTCStar: { // <- note the upper-case w- see https://github.com/libp2p/js-libp2p/issues/576
                            wrtc
                        }
                    }
                }
            },

            // preload: {
            //     enabled: false
            // },
            relay: { enabled: true, hop: { enabled: true, active: true } },
            repo: './.data/ipfs',
            start: true
        })
        ipfs.libp2p.connectionManager.on('peer:connect', async ipfsPeer => {
            //console.log('IPFS peer connected', ipfsPeer.id)
            //console.log('Peers', (await ipfs.swarm.peers()).length)
            //console.log(ipfsPeer)
        })

        const identity = await ipfs.id()
        console.debug(`IPFS: Node ${identity.id} created.`)
        console.log(identity.addresses)

        // const result = await ipfs.ping(identity.id)
        // console.log(result)

        // let bootstrapAddresses = await ipfs.bootstrap.list()
        // console.log(bootstrapAddresses.Peers)
        //
        // const swarmPeers = await ipfs.swarm.peers()
        // console.log(swarmPeers)

        //console.log('Peers', (await ipfs.swarm.peers()).length)

        console.log("Creating OrbitDB instance...")
        const orbitdb = await OrbitDB.createInstance(ipfs, { directory: './.data/orbitdb' })

        console.log("Creating database...")
        const options = { accessController: { write: ['*'] } }
        const address = await orbitdb.determineAddress(database, 'eventlog',options)
        const db = (address === null)
            ? await orbitdb.create(database, 'eventlog', options)
            : await orbitdb.open(address)
        db.events.on('peer', async () => console.log('peer connected'))
        db.events.on('peer.exchanged', async () => console.log('peer exchanged'))
        db.events.on('replicated', async (address) => {
            console.debug('replicated', address)
        })
        db.events.on('replicate', async (address) => {
            console.debug('replicate', address)
        })
        db.events.on('replicate.progress', async (address, hash, entry, progress, have) => {
            console.debug('replicate.progress', address, hash, entry, progress, have)
        })
        db.events.on('load', async (dbname) => {
            console.debug('load', dbname)
        })
        db.events.on('load.progress', async (address, hash, entry, progress, total) => {
            console.debug('load.progress', address, hash, entry, progress, total)
        })
        db.events.on('ready', async (dbname, heads) => {
            console.debug('ready', dbname, heads)
        })
        db.events.on('write', async (address, entry, heads) => {
            console.debug('write', address, entry, heads)
        })
        db.events.on('closed', async (dbname) => {
            console.debug('closed', dbname)
        })
        db.events.on('peer', async (peer) => {
            console.debug('peer', peer)
        })
        db.events.on('peer.exchanged', async (peer, address, heads)=> {
            console.debug('peer.exchanged', peer, address, heads)
        })

        console.log(`Connected to database ${db.id}`)

        console.log("Loading database...")
        await db.load()

        await db.add('test')

        const subscriptions = await ipfs.pubsub.ls()
        for (let subscription of subscriptions) {
            console.log(subscription)
        }

        console.log("Monitoring changes...")
    } catch (e) {
        console.error(e)
        process.exit(1)
    }

    // const query = async () => {
    //     const index = Math.floor(Math.random() * creatures.length)
    //     const userId = Math.floor(Math.random() * 900 + 100)
    //
    //     try {
    //         await db.add({ avatar: creatures[index], userId: userId })
    //         const latest = db.iterator({ limit: 5 }).collect()
    //         let output = ``
    //         output += `[Latest Visitors]\n`
    //         output += `--------------------\n`
    //         output += `ID  | Visitor\n`
    //         output += `--------------------\n`
    //         output += latest.reverse().map((e) => e.payload.value.userId + ' | ' + e.payload.value.avatar).join('\n') + `\n`
    //         console.log(output)
    //     } catch (e) {
    //         console.error(e)
    //         process.exit(1)
    //     }
    // }
    //
    // setInterval(query, 1000)
}
main(randomAddress(10))



function randomAddress(length) {
    let result = ''
    const characters = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789'
    const charactersLength = characters.length
    for ( let i = 0; i < length; i++ ) {
        result += characters.charAt(Math.floor(Math.random() *
            charactersLength))
    }
    return result
}