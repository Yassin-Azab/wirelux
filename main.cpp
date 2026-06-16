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

struct AppStats {
    uint64_t    bytes_sent     = 0;
    uint64_t    bytes_received = 0;
//ADD IP, SOURCE PORT, DESTINATION PORT???
    uint64_t total() const {
        return bytes_sent + bytes_received;
    }
};

void packethandler(u_char* user, const struct pcap_pkthdr* header, const u_char* packet){
if (header->caplen < 34) return;
auto* app_data = reinterpret_cast<std::map<std::string, std::pair<AppStats, TimePoint>>*>(user);
const struct ethhdr* eth =reinterpret_cast<const struct ethhdr*>(packet);
if (ntohs(eth->h_proto) != ETH_P_IP) return;
const struct iphdr* ip = reinterpret_cast<const struct iphdr*>(packet + 14);
if ((ip->protocol != IPPROTO_TCP && ip->protocol != IPPROTO_UDP)|| ip->version != 4||ip->ihl < 5) return;
const struct u_char* transport = packet + 14 + ip->ihl*4;
if (ip->protocol == IPPROTO_TCP) { 
const struct tcphdr* tcp = reinterpret_cast<const struct tcphdr*>(transport);
}
if (ip->protocol == IPPROTO_UDP) { 
const struct udphdr* udp = reinterpret_cast<const struct udphdr*>(transport);
}


}
pcap_t* g_handle = nullptr;  
using TimePoint = std::chrono::time_point<std::chrono::system_clock>;
std::map<std::string, std::pair<AppStats, TimePoint>> AppData;



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

int pcap_loop(g_handle, -1, &packethandler, reinterpret_cast<u_char*>(&AppData));
signal(SIGINT, on_ctrl_c);
void pcap_breakloop(pcap_t *handle);
void pcap_freealldevs(pcap_if_t *AllInterfaces);
pcap_close(g_handle);
AllInterfaces=nullptr;
g_handle = nullptr;
return 0;
}