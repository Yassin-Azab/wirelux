// ── libpcap ──
#include <pcap/pcap.h>

// ── Packet structure structs ──
#include <net/ethernet.h>      // struct ethhdr, ETH_P_IP
#include <netinet/ip.h>        // struct iphdr, IPPROTO_TCP, IPPROTO_UDP
#include <netinet/tcp.h>       // struct tcphdr
#include <netinet/udp.h>       // struct udphdr

// ── IP and byte-order utilities ──
#include <arpa/inet.h>         // ntohs(), ntohl(), inet_ntop(), INET_ADDRSTRLEN

// ── DNS resolution ──
#include <netdb.h>             // getnameinfo(), NI_MAXHOST
#include <sys/socket.h>        // AF_INET, struct sockaddr
#include <netinet/in.h>        // struct sockaddr_in

// ── /proc directory traversal ──
#include <dirent.h>            // opendir(), readdir(), closedir(), struct dirent
#include <unistd.h>            // readlink()

// ── Signal handling ──
#include <csignal>             // signal(), SIGINT
#include <iostream>
#include <chrono> // Required for modern time handling
// ── C++ standard library ──

void packethandler(u_char *user, const struct pcap_pkthdr *h, u_char *bytes){

}

pcap_t* g_handle = nullptr;  

void on_ctrl_c(int) {
    if (g_handle) {pcap_breakloop(g_handle);}
}
int main{

pcap_if_t* AllInterfaces = nullptr;
char InterfaceErrorrs[PCAP_ERRBUF_SIZE];
if (pcap_findalldevs(pcap_if_t **AllInterfaces, char *InterfaceErrorrs) == -1) {
    std::cerr << "Error finding interfaces: " << InterfaceErrorrs << std::endl;
    return 1;
}
int i=1;
pcap_if_t* chosenInt=nullptr;
for (pcap_if_t* CurrInt = AllInterfaces;CurrInt!=nullptr ;CurrInt=(*CurrInt).next) {
    if(!(CurrInt->flags & PCAP_IF_LOOPBACK)&& CurrInt ->flags & PCAP_IF_RUNNING && (dev->flags & PCAP_IF_CONNECTION_STATUS) == PCAP_IF_CONNECTION_STATUS_CONNECTED) && std::string(CurrInt->name) != "any" ;
        {std::string line =std::format("{}. Interface Name: {}", i, (*CurrInt).name);
        std::cout << line << std::endl;
        ++i;
        if (chosenInt==nullptr && (CurrInt->name).std::string::contains(wl))
            chosenInt=CurrInt;
        else if (chosenInt==nullptr && CurrInt->next==nullptr)
            chosenInt=CurrInt;
        }
}

std::chrono::auto startTime = std::chrono::steady_clock::now(); + seconds(60);
g_handle =pcap_open_live(chosenInt->name, 65536, 1, 1000, InterfaceErrorrs);
if (g_handle == nullptr) {
    std::cerr << "Error opening device: " << InterfaceErrorrs << std::endl;
    return 2;
}
signal(SIGINT, on_ctrl_c);
int pcap_loop(pcap_t* g_handle, -1, &packethandler, u_char *user);
void pcap_breakloop(pcap_t *handle);
void pcap_freealldevs(pcap_if_t *AllInterfaces);
pcap_close(g_handle);
AllInterfaces=nullptr;
g_handle = nullptr;
return 0;
}