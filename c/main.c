#include <netinet/in.h>
#include <stdio.h>
#include <sys/socket.h>
#include <unistd.h>

int main(void) {
    printf("Hello, I am Simple C Server!\n");

    int port;
    char buffer[256];
    struct sockaddr_in server;
    struct sockaddr_in client;
    int s;
    int ns;
    socklen_t namelen;

    port = 5000;

    s = socket(AF_INET, SOCK_STREAM, 0);
    if (s < 0) {
        perror("socket");
        return 1;
    }

    server.sin_family = AF_INET;
    server.sin_port = htons(port);
    server.sin_addr.s_addr = INADDR_ANY;

    if (bind(s, (struct sockaddr *)&server, sizeof(server)) < 0) {
        perror("bind");
        return 1;
    }

    int l = listen(s, 5);
    if (l != 0) {
        perror("listen");
        return 1;
    }

    while (1) {
        namelen = sizeof(client);
        ns = accept(s, (struct sockaddr *)&client, &namelen);
        if (ns == -1) {
            perror("accept");
            return 1;
        }

        int rcv = recv(ns, buffer, sizeof(buffer), 0);
        if (rcv == -1) {
            perror("recv");
            return 1;
        }

        printf("Received: %s\n", buffer);

        int snd = send(ns, buffer, rcv, 0);
        if (snd == -1) {
            perror("send");
            return 1;
        }

        close(ns);
    }

    close(s);
    printf("%s\n", "Simple Server is exiting!");

    return 0;
}
