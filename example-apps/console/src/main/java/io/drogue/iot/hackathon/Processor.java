package io.drogue.iot.hackathon;

import java.util.UUID;

import javax.enterprise.context.ApplicationScoped;
import javax.inject.Inject;
import javax.transaction.Transactional;
import javax.ws.rs.WebApplicationException;

import org.eclipse.microprofile.reactive.messaging.Channel;
import org.eclipse.microprofile.reactive.messaging.Emitter;
import org.eclipse.microprofile.reactive.messaging.Incoming;
import org.eclipse.microprofile.reactive.messaging.OnOverflow;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import io.drogue.iot.hackathon.data.CommandPayload;
import io.drogue.iot.hackathon.data.DeviceEvent;
import io.drogue.iot.hackathon.data.DisplaySettings;
import io.drogue.iot.hackathon.data.OnOffSet;
import io.drogue.iot.hackathon.events.EventDispatcher;
import io.drogue.iot.hackathon.integration.DeviceCommand;
import io.drogue.iot.hackathon.registry.Registry;
import io.drogue.iot.hackathon.service.DeviceClaimService;
import io.drogue.iot.hackathon.service.StillClaimedException;
import io.quarkus.runtime.Startup;
import io.smallrye.reactive.messaging.annotations.Broadcast;

/**
 * Process device events.
 * <p>
 * This is main logic in this application. Processing happens in the {@link #process(DeviceEvent)} method.
 * <p>
 * It receives messages from the Drogue IoT MQTT integration, pre-processed by the {@link
 * io.drogue.iot.hackathon.integration.Receiver}. It can return {@code null} to do nothing, or a {@link CommandPayload} to
 * send a response back to the device.
 * <p>
 * As this targets a LoRaWAN use case, where the device sends an uplink (device-to-cloud) message, and waits a very
 * short period of time for a downlink (cloud-to-device) message, we must act quickly, and directly respond. We still
 * can send a command to the device the same way at any time. The message might get queued if it cannot be delivered
 * right away. But for this demo, we want to see some immediate results.
 */
@Startup
@ApplicationScoped
public class Processor {

    private static final Logger LOG = LoggerFactory.getLogger(Processor.class);

    @Inject
    @Channel("device-commands")
    @Broadcast
    @OnOverflow(value = OnOverflow.Strategy.LATEST)
    Emitter<DeviceCommand> deviceCommands;

    public void displayCommand(DisplaySettings settings) {

        var display = new OnOffSet(settings.enabled);
        display.setLocation((short) 0x100);
        var commandPayload = new CommandPayload(settings.address, display);
        var command = new DeviceCommand();

        command.setDeviceId(settings.device);
        command.setPayload(commandPayload);

        LOG.info("Sending command: {} to address {}", command, settings.device);

        this.deviceCommands.send(command);
    }

    @Incoming("event-stream")
    @OnOverflow(value = OnOverflow.Strategy.DROP)
    public void process(DeviceEvent event) {
        var payload = event.getPayload();

        LOG.info("Received sensor data: {}", payload);
    }

    @Inject
    Registry registry;

    @Inject
    DeviceClaimService service;

    @Inject
    EventDispatcher dispatcher;

    @Transactional
    public void claimDevice(final String claimId, final String userId, final boolean canCreate) {
        this.service.claimDevice(claimId, userId, canCreate);
    }

    @Transactional
    public void claimSimulatorDevice(final String userId) {
        var id = "simulator-" + UUID.randomUUID();
        var pwd = UUID.randomUUID().toString();
        this.service.claimDevice(id, userId, true);
        this.registry.createSimulatorDevice(id, pwd);
    }

    @Transactional
    public void releaseDevice(final String userId) {
        var claim = this.service.getDeviceClaimFor(userId);
        claim.ifPresent(deviceClaim -> {
            if (deviceClaim.getId().startsWith("simulator-")) {
                try {
                    this.registry.deleteDevice(deviceClaim.getId());
                } catch (WebApplicationException e) {
                    if (e.getResponse().getStatus() != 404) {
                        // ignore 404
                        throw e;
                    }
                }
            }
            this.dispatcher.releaseDevice(deviceClaim.getId());
        });

        this.service.releaseDevice(userId);
    }
}
