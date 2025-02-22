package io.drogue.iot.hackathon;

import io.quarkus.runtime.QuarkusApplication;
import io.quarkus.runtime.annotations.QuarkusMain;
import io.quarkus.runtime.Quarkus;

import java.net.URI;
import java.util.concurrent.LinkedBlockingDeque;

import javax.websocket.ClientEndpoint;
import javax.websocket.ContainerProvider;
import javax.websocket.OnMessage;
import javax.websocket.OnOpen;
import javax.websocket.Session;

import org.eclipse.microprofile.config.inject.ConfigProperty;
import org.jboss.logging.Logger;


@QuarkusMain
public class WebSocketEvents implements QuarkusApplication {

    // This is just a convenience method so we can run this app from the IDE
    public static void main(String... args) {
        Quarkus.run(WebSocketEvents.class, args);
    }

    @Override
    public int run(String... args) throws Exception {
        connectToWebSocket();
        return 0;
    }


    private static final LinkedBlockingDeque<String> MESSAGES = new LinkedBlockingDeque<>();
    private static final Logger LOG = Logger.getLogger(SocketClient.class);

    @ConfigProperty(name = "greeting.suffix", defaultValue="wss")
    String protocol;
    @ConfigProperty(name = "drogue.integration.websocket.host")
    String host = "ws-integration.sandbox.drogue.cloud";
    @ConfigProperty(name = "drogue.integration.websocket.port", defaultValue="443")
    int port;
    @ConfigProperty(name = "drogue.application.name")
    String applicationName;
    @ConfigProperty(name = "drogue.api.user")
    String username;
    @ConfigProperty(name = "drogue.api.key")
    String key;

    public void connectToWebSocket() throws Exception {
        String url = String.format("%s://%s:%d/%s?username=%s&api_key=%s", protocol, host, port, applicationName, username, key);
        URI endpoint = URI.create(url);
        System.out.println(String.format("connecting to %s", url));
        ContainerProvider.getWebSocketContainer().connectToServer(SocketClient.class, endpoint);
    }


    @ClientEndpoint
    public static class SocketClient {
        private static final LinkedBlockingDeque<String> MESSAGES = new LinkedBlockingDeque<>();
        private static final Logger LOG = Logger.getLogger(SocketClient.class);

        @OnOpen
        public void open(Session session) {
            MESSAGES.add("CONNECT");
        }

        @OnMessage
        void message(String msg) {
            System.out.println(msg);
        }
    }
}
